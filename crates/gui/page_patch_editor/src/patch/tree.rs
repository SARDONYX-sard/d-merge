//! Object tree logic (drawn by [`super::ui::tree`]): from the root (`hkRootLevelContainer`) down
//! the `#XXXX` pointers in each object.
//!
//! A behavior file is a graph of objects. Each object is an XML block whose text mentions other
//! objects by id (see [`pointers`]), so following those mentions from the root gives a tree:
//!
//! ```text
//! ▼ #0090 hkRootLevelContainer
//!   ▼ #0091 hkbBehaviorGraph  BehaviorGraph
//!     ▶ #0094 hkbBehaviorGraphData
//!     ▶ #0095 hkbStateMachine  Root
//!       #0097 hkbClipGenerator    (dimmed: expanded at another place above)
//! ```
//! Objects can have several parents and pointers can form cycles, so each object only expands
//! under the first parent a depth-first walk meets ([`Graph::first_parent`]); elsewhere it is a
//! dimmed leaf.
//!
//! # Why it is split into [`Graph`] and rows
//! A behavior has thousands of objects and this is drawn every frame.
//! - [`Graph`]: the whole file, rebuilt only after an edit. It knows what is reachable no matter
//!   what is expanded, so the "not linked" count does not depend on the tree's open nodes.
//! - rows ([`TreeState::rows`]): the currently expanded nodes as a flat list, rebuilt only when
//!   something changes. Only the rows on screen are drawn.

use std::rc::Rc;

use fast_hash::{HashMap, HashSet};

use super::workspace::{Behavior, ObjectText};

/// Index of an object in [`Graph`]. Only valid for the graph it came from (edits renumber).
pub(super) type Node = u32;

/// The pointers between all objects of a behavior. Rebuilt after every edit.
#[derive(Debug, Default)]
pub(super) struct Graph {
    /// New objects first, then the template's, e.g. `["#mymod$0", "#0090", "#0091"]`.
    ids: Vec<Rc<str>>,
    index: HashMap<Rc<str>, Node>,
    /// e.g. `● #0106  hkbModifierList  GetUpModList`, `+ #mymod$0  hkbClipGenerator  ← #0106`
    labels: Vec<Box<str>>,
    /// Where the text after the id starts in each label (`hkbModifierList  GetUpModList`), which
    /// is also the description of the object in hovers and completion.
    doc_starts: Vec<u32>,
    children: Vec<Vec<Node>>,
    /// The parent an object expands under (`None`: the root, or not reachable from it).
    first_parent: Vec<Option<Node>>,
    /// Edited or new.
    changed: Vec<bool>,
    /// New objects are `0..new_objects`.
    new_objects: usize,
    /// Changed, or (transitively) points to a changed object: the "changed only" tree.
    relevant: Vec<bool>,
    root: Option<Node>,
    /// New objects the root does not reach: Nemesis would never use them.
    unlinked: Vec<Node>,
}

impl Graph {
    pub(super) fn new(behavior: &Behavior) -> Self {
        let objects: Vec<_> = behavior.texts().collect();
        let root = behavior
            .object_of_class("hkRootLevelContainer")
            .or_else(|| behavior.objects.first().map(|o| o.id.as_str()));
        Self::from_objects(&objects, root)
    }

    /// `objects` must have the new ones first ([`Behavior::texts`]).
    fn from_objects(objects: &[ObjectText<'_>], root: Option<&str>) -> Self {
        let ids: Vec<Rc<str>> = objects.iter().map(|object| Rc::from(object.id)).collect();
        let index: HashMap<Rc<str>, Node> = ids.iter().enumerate().map(|(i, id)| (Rc::clone(id), node(i))).collect();
        let children: Vec<Vec<Node>> = objects
            .iter()
            .map(|object| pointers(object.text, object.id).iter().filter_map(|id| index.get(*id).copied()).collect())
            .collect();
        let changed: Vec<bool> = objects.iter().map(|object| object.is_new || object.is_edited).collect();
        let new_objects = objects.iter().filter(|object| object.is_new).count();
        let root = root.and_then(|id| index.get(id).copied());

        let (reachable, first_parent) = walk_from(root, &children);
        let relevant = relevant_nodes(&children, &changed);
        let (labels, doc_starts) = object_labels(objects, &children, new_objects);
        let unlinked = (0..new_objects).filter(|&i| !reachable[i]).map(node).collect();

        Self { ids, index, labels, doc_starts, children, first_parent, changed, new_objects, relevant, root, unlinked }
    }

    pub(super) fn id(&self, node: Node) -> &str {
        &self.ids[node as usize]
    }

    pub(super) fn label(&self, node: Node) -> &str {
        &self.labels[node as usize]
    }

    /// e.g. `hkbModifierList  GetUpModList`
    pub(super) fn doc(&self, node: Node) -> &str {
        &self.labels[node as usize][self.doc_starts[node as usize] as usize..]
    }

    pub(super) fn node_of(&self, id: &str) -> Option<Node> {
        self.index.get(id).copied()
    }

    /// Every object, new ones first.
    pub(super) fn nodes(&self) -> impl Iterator<Item = Node> {
        (0..self.ids.len()).map(node)
    }

    pub(super) fn is_changed(&self, node: Node) -> bool {
        self.changed[node as usize]
    }

    pub(super) const fn is_new(&self, node: Node) -> bool {
        (node as usize) < self.new_objects
    }

    pub(super) fn unlinked(&self) -> &[Node] {
        &self.unlinked
    }

    /// `node`'s children in the tree being shown.
    fn shown_children(&self, node: Node, changed_only: bool) -> impl Iterator<Item = Node> {
        self.children[node as usize]
            .iter()
            .copied()
            .filter(move |&child| !changed_only || self.relevant[child as usize])
    }
}

/// `usize` -> [`Node`]. (A behavior has far fewer than 4 billion objects.)
#[expect(clippy::cast_possible_truncation, reason = "see above")]
const fn node(index: usize) -> Node {
    index as Node
}

/// What the root reaches, and the parent each object was first reached from.
fn walk_from(root: Option<Node>, children: &[Vec<Node>]) -> (Vec<bool>, Vec<Option<Node>>) {
    fn visit(node: Node, children: &[Vec<Node>], reachable: &mut [bool], first_parent: &mut [Option<Node>]) {
        reachable[node as usize] = true;
        for &child in &children[node as usize] {
            if !reachable[child as usize] {
                first_parent[child as usize] = Some(node);
                visit(child, children, reachable, first_parent);
            }
        }
    }
    let mut reachable = vec![false; children.len()];
    let mut first_parent = vec![None; children.len()];
    if let Some(root) = root {
        visit(root, children, &mut reachable, &mut first_parent);
    }
    (reachable, first_parent)
}

/// For each object: is it changed, or does it (transitively) point to a changed object?
fn relevant_nodes(children: &[Vec<Node>], changed: &[bool]) -> Vec<bool> {
    fn visit(node: Node, children: &[Vec<Node>], changed: &[bool], memo: &mut [Option<bool>]) -> bool {
        if let Some(known) = memo[node as usize] {
            return known;
        }
        memo[node as usize] = Some(false); // a cycle back here adds nothing
        let mut relevant = changed[node as usize];
        for &child in &children[node as usize] {
            relevant |= visit(child, children, changed, memo);
        }
        memo[node as usize] = Some(relevant);
        relevant
    }
    let mut memo = vec![None; children.len()];
    (0..children.len()).map(|i| visit(node(i), children, changed, &mut memo)).collect()
}

/// The list text of every object, and where its description starts. See [`Graph::labels`].
fn object_labels(objects: &[ObjectText<'_>], children: &[Vec<Node>], new_objects: usize) -> (Vec<Box<str>>, Vec<u32>) {
    // Who points to each new object: its label tells where it is linked.
    let mut linked_from: Vec<Vec<&str>> = vec![Vec::new(); new_objects];
    for (i, targets) in children.iter().enumerate() {
        for &target in targets {
            if let Some(from) = linked_from.get_mut(target as usize) {
                from.push(objects[i].id);
            }
        }
    }

    let mut labels = Vec::with_capacity(objects.len());
    let mut doc_starts = Vec::with_capacity(objects.len());
    for (i, object) in objects.iter().enumerate() {
        let marker = if object.is_new {
            '+'
        } else if object.is_edited {
            '●'
        } else {
            ' '
        };
        let head = format!("{marker} {}  ", object.id);
        let (class, name) = (object.class, field_name(object.text).unwrap_or(object.name));
        let tail = match linked_from.get(i) {
            Some(from) if from.is_empty() => format!("{class}  ⚠ not linked"),
            Some(from) => format!("{class}  {name} ← {}", from.join(", ")),
            None => format!("{class}  {name}"),
        };
        #[expect(clippy::cast_possible_truncation, reason = "an id is far shorter than 4 GiB")]
        doc_starts.push(head.len() as u32);
        labels.push(format!("{head}{tail}").into_boxed_str());
    }
    (labels, doc_starts)
}

/// A line of the tree on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) node: Node,
    pub(super) depth: u16,
    pub(super) expandable: bool,
    pub(super) open: bool,
    /// Expanded at another place.
    pub(super) dimmed: bool,
}

/// A tree command, whichever key asked for it. The keys are in [`super::ui::tree::read_keys`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TreeInput {
    /// `j`, Down
    Next,
    /// `k`, Up
    Previous,
    /// `g`
    First,
    /// `G`
    Last,
    /// `h`, Left: close the node, or (already closed) go to its parent.
    Close,
    /// `l`, Right: open the node, or (already open) go into it.
    Open,
    /// `Enter`, `Esc`, `Space e`: back to the editor.
    Leave,
}

impl TreeInput {
    /// The row a movement leads to in a list of `len` rows (`None`: not a movement).
    ///
    /// e.g., `Next` on the last row stays there.
    pub(super) const fn step(self, current: usize, len: usize) -> Option<usize> {
        let last = len.saturating_sub(1);
        match self {
            Self::Next => Some(if current < last { current + 1 } else { last }),
            Self::Previous => Some(current.saturating_sub(1)),
            Self::First => Some(0),
            Self::Last => Some(last),
            Self::Close | Self::Open | Self::Leave => None,
        }
    }
}

/// What a [`TreeInput`] leads to. See [`apply_input`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    Cursor(usize),
    SetOpen(Node, bool),
    Leave,
    None,
}

/// The effect of `input` with the cursor on row `cursor`.
fn apply_input(input: TreeInput, rows: &[Row], cursor: usize) -> Effect {
    let cursor = cursor.min(rows.len().saturating_sub(1));
    if let Some(row) = input.step(cursor, rows.len()) {
        return Effect::Cursor(row);
    }
    let row = rows.get(cursor);
    match input {
        TreeInput::Close => match row {
            Some(row) if row.open => Effect::SetOpen(row.node, false),
            Some(row) => rows[..cursor].iter().rposition(|r| r.depth < row.depth).map_or(Effect::None, Effect::Cursor),
            None => Effect::None,
        },
        TreeInput::Open => match row {
            Some(row) if row.open => Effect::Cursor(TreeInput::Next.step(cursor, rows.len()).unwrap_or(cursor)),
            Some(row) if row.expandable => Effect::SetOpen(row.node, true),
            _ => Effect::None,
        },
        TreeInput::Leave => Effect::Leave,
        TreeInput::Next | TreeInput::Previous | TreeInput::First | TreeInput::Last => Effect::None,
    }
}

/// Tree state kept between frames.
#[derive(Debug, Default)]
pub(super) struct TreeState {
    /// Nodes the user opened / closed, by id so it survives edits. Others: [`Self::default_open`].
    open: HashMap<Rc<str>, bool>,
    rows: Vec<Row>,
    /// What `rows` was built from: the graph and "changed only".
    built_for: Option<(Rc<Graph>, bool)>,
    /// The row of the selected object.
    pub(super) cursor: usize,
    /// The selection `rows` was revealed for (to notice a selection made elsewhere, e.g. Vim `gd`).
    revealed: Option<Rc<str>>,
    /// Bring this row into view on the next draw.
    pub(super) scroll_to: Option<usize>,
    pub(super) scroll_offset: f32,
    pub(super) view_height: f32,
    /// `Space` was pressed: the next `e` leaves the tree (same keys as entering it).
    pub(super) leader: bool,
}

impl TreeState {
    pub(super) fn rows(&self) -> &[Row] {
        &self.rows
    }

    fn default_open(graph: &Graph, node: Node, changed_only: bool) -> bool {
        graph.root == Some(node) || changed_only
    }

    fn is_open(&self, graph: &Graph, node: Node, changed_only: bool) -> bool {
        self.open.get(graph.id(node)).copied().unwrap_or_else(|| Self::default_open(graph, node, changed_only))
    }

    pub(super) fn set_open(&mut self, graph: &Graph, node: Node, open: bool) {
        self.open.insert(Rc::clone(&graph.ids[node as usize]), open);
        self.built_for = None;
    }

    /// Opens / closes `node` and everything expanding under it.
    pub(super) fn set_open_below(&mut self, graph: &Graph, node: Node, open: bool) {
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            self.set_open(graph, current, open);
            stack.extend(
                graph.children[current as usize].iter().filter(|&&c| graph.first_parent[c as usize] == Some(current)),
            );
        }
    }

    /// Rebuilds the rows if needed and shows `selected` if it changed (opens its parents).
    pub(super) fn prepare(&mut self, graph: &Rc<Graph>, selected: Option<&str>, changed_only: bool) {
        let revealed = self.reveal(graph, selected, changed_only);
        let stale =
            self.built_for.as_ref().is_none_or(|(built, only)| !Rc::ptr_eq(built, graph) || *only != changed_only);
        if stale {
            self.rebuild_rows(graph, changed_only);
        }
        if revealed || stale {
            self.follow_selection(graph, selected, revealed);
        }
    }

    /// Opens the parents of `selected` if it is a new selection. Returns whether it was.
    fn reveal(&mut self, graph: &Graph, selected: Option<&str>, changed_only: bool) -> bool {
        let new_selection = selected.filter(|id| self.revealed.as_deref() != Some(*id));
        let Some((id, &target)) = new_selection.and_then(|id| graph.index.get_key_value(id)) else { return false };
        self.revealed = Some(Rc::clone(id));
        let mut parent = graph.first_parent[target as usize];
        while let Some(node) = parent {
            if !self.is_open(graph, node, changed_only) {
                self.set_open(graph, node, true);
            }
            parent = graph.first_parent[node as usize];
        }
        true
    }

    fn rebuild_rows(&mut self, graph: &Rc<Graph>, changed_only: bool) {
        let mut rows = std::mem::take(&mut self.rows);
        rows.clear();
        if let Some(root) = graph.root {
            self.push_rows(graph, root, 0, changed_only, &mut rows);
        }
        self.rows = rows;
        self.built_for = Some((Rc::clone(graph), changed_only));
    }

    fn push_rows(&self, graph: &Graph, node: Node, depth: u16, changed_only: bool, rows: &mut Vec<Row>) {
        let expandable = graph.shown_children(node, changed_only).next().is_some();
        let open = expandable && self.is_open(graph, node, changed_only);
        rows.push(Row { node, depth, expandable, open, dimmed: false });
        if !open {
            return;
        }
        for child in graph.shown_children(node, changed_only) {
            if graph.first_parent[child as usize] == Some(node) {
                self.push_rows(graph, child, depth + 1, changed_only, rows);
            } else {
                rows.push(Row { node: child, depth: depth + 1, expandable: false, open: false, dimmed: true });
            }
        }
    }

    /// Puts the cursor on the selected object's row, and scrolls to it if `scroll`.
    fn follow_selection(&mut self, graph: &Graph, selected: Option<&str>, scroll: bool) {
        let node = selected.and_then(|id| graph.node_of(id));
        let row = node.and_then(|node| self.rows.iter().position(|r| r.node == node && !r.dimmed));
        if let Some(row) = row {
            self.cursor = row;
            if scroll {
                self.scroll_to = Some(row);
            }
        }
    }

    /// Runs the keys of this frame. Returns `true` if the keyboard goes back to the editor.
    pub(super) fn apply_inputs(
        &mut self,
        inputs: &[TreeInput],
        graph: &Rc<Graph>,
        selected: &mut Option<String>,
        changed_only: bool,
    ) -> bool {
        self.prepare(graph, selected.as_deref(), changed_only);
        let previous = self.cursor;
        let mut leave = false;
        for &input in inputs {
            match apply_input(input, &self.rows, self.cursor) {
                Effect::Cursor(cursor) => self.cursor = cursor,
                Effect::SetOpen(node, open) => {
                    self.set_open(graph, node, open);
                    self.rebuild_rows(graph, changed_only);
                }
                Effect::Leave => leave = true,
                Effect::None => {}
            }
        }
        if self.cursor != previous {
            self.select_cursor(graph, selected);
        }
        leave
    }

    /// Opens the object of the cursor's row, as `j` / `k` do.
    fn select_cursor(&mut self, graph: &Graph, selected: &mut Option<String>) {
        self.scroll_to = Some(self.cursor);
        if let Some(row) = self.rows.get(self.cursor) {
            let id = graph.id(row.node);
            self.revealed = graph.index.get_key_value(id).map(|(id, _)| Rc::clone(id));
            id.clone_into(selected.get_or_insert_default());
        }
    }

    /// The scroll offset that brings row `row` into view (`None`: it already is).
    pub(super) fn offset_for(&self, row: usize, row_height: f32) -> Option<f32> {
        #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
        let top = row as f32 * row_height;
        if top < self.scroll_offset {
            Some(top)
        } else if top + row_height > self.scroll_offset + self.view_height {
            Some(top + row_height - self.view_height)
        } else {
            None
        }
    }
}

/// The value of the object's `name` field.
///
/// e.g., `<hkparam name="name">GetUpModList</hkparam>` -> `GetUpModList`
fn field_name(text: &str) -> Option<&str> {
    let (_, rest) = text.split_once("<hkparam name=\"name\">")?;
    rest.split_once("</hkparam>").map(|(name, _)| name)
}

/// The `#XXXX` / `#mod$N` pointers of an object (not its own name), first occurrence order.
///
/// e.g., `name="#0106"`, own = `#0106`, then `#0105 #0109 #0105` -> `["#0105", "#0109"]`
fn pointers<'a>(text: &'a str, own: &str) -> Vec<&'a str> {
    let is_id_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut found: Vec<&str> = Vec::new();
    // A generator can list hundreds of children: a set keeps this linear.
    let mut seen: HashSet<&str> = HashSet::default();
    let mut rest = text;
    while let Some(at) = rest.find('#') {
        let len = rest[at + 1..].find(|c: char| !is_id_char(c)).unwrap_or(rest.len() - at - 1);
        let token = &rest[at..at + 1 + len];
        if len > 0 && token != own && seen.insert(token) {
            found.push(token);
        }
        rest = &rest[at + 1 + len..];
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object<'a>(id: &'a str, text: &'a str, is_new: bool) -> ObjectText<'a> {
        ObjectText { id, class: "c", name: "", text, is_new, is_edited: false }
    }

    /// `#new0` is reached through `#b`, `#new1` through nothing. `#c` is shared by `#a` and `#b`.
    fn sample() -> Rc<Graph> {
        let objects = [
            object("#new0", "", true),
            object("#new1", "", true),
            object("#root", "#a #b", false),
            object("#a", "#c", false),
            object("#b", "#c #new0", false),
            object("#c", "", false),
        ];
        Rc::new(Graph::from_objects(&objects, Some("#root")))
    }

    fn ids<'a>(graph: &'a Graph, state: &TreeState) -> Vec<&'a str> {
        state.rows().iter().map(|row| graph.id(row.node)).collect()
    }

    #[test]
    fn finds_pointers() {
        let text = "<hkobject name=\"#0106\" class=\"x\">\n#0105 #0109\n#mymod$0 #0105\n</hkobject>\n";
        assert_eq!(pointers(text, "#0106"), ["#0105", "#0109", "#mymod$0"]);
    }

    /// The "not linked" count must not depend on what is expanded.
    #[test]
    fn unlinked_does_not_depend_on_open_nodes() {
        let graph = sample();
        let unlinked: Vec<_> = graph.unlinked().iter().map(|&node| graph.id(node)).collect();
        assert_eq!(unlinked, ["#new1"]);

        let mut state = TreeState::default();
        state.prepare(&graph, None, false);
        assert_eq!(ids(&graph, &state), ["#root", "#a", "#b"]); // only the root starts open
        assert_eq!(graph.unlinked().len(), 1);
    }

    #[test]
    fn labels_tell_where_new_objects_are_linked() {
        let graph = sample();
        assert_eq!(graph.label(0), "+ #new0  c   ← #b");
        assert_eq!(graph.doc(0), "c   ← #b");
        assert_eq!(graph.label(1), "+ #new1  c  ⚠ not linked");
    }

    /// `#c` has two parents: it expands under the first one and is dimmed under the other.
    #[test]
    fn shared_objects_expand_once() {
        let graph = sample();
        let mut state = TreeState::default();
        state.prepare(&graph, None, false);
        let root = graph.node_of("#root").unwrap();
        state.set_open_below(&graph, root, true);
        state.prepare(&graph, None, false);

        assert_eq!(ids(&graph, &state), ["#root", "#a", "#c", "#b", "#c", "#new0"]);
        let dimmed: Vec<_> = state.rows().iter().map(|row| row.dimmed).collect();
        assert_eq!(dimmed, [false, false, false, false, true, false]);
    }

    #[test]
    fn selecting_opens_the_parents() {
        let graph = sample();
        let mut state = TreeState::default();
        state.prepare(&graph, Some("#new0"), false);
        assert_eq!(ids(&graph, &state), ["#root", "#a", "#b", "#c", "#new0"]);
        assert_eq!(state.cursor, 4);
        assert_eq!(state.scroll_to, Some(4));
    }

    #[test]
    fn keys_move_and_select() {
        let graph = sample();
        let mut state = TreeState::default();
        let mut selected = None;
        let leave = state.apply_inputs(&[TreeInput::Next, TreeInput::Next], &graph, &mut selected, false);
        assert!(!leave);
        assert_eq!(selected.as_deref(), Some("#b"));
        assert!(state.apply_inputs(&[TreeInput::Leave], &graph, &mut selected, false));
    }

    #[test]
    fn input_effects() {
        let row = |node, depth, open, expandable| Row { node, depth, expandable, open, dimmed: false };
        // 0 (open) > 1 (closed, has children), 2 (leaf)
        let rows = [row(0, 0, true, true), row(1, 1, false, true), row(2, 1, false, false)];

        assert_eq!(apply_input(TreeInput::Next, &rows, 0), Effect::Cursor(1));
        assert_eq!(apply_input(TreeInput::Next, &rows, 2), Effect::Cursor(2)); // stops at the end
        assert_eq!(apply_input(TreeInput::Previous, &rows, 0), Effect::Cursor(0));
        assert_eq!(apply_input(TreeInput::Last, &rows, 0), Effect::Cursor(2));

        assert_eq!(apply_input(TreeInput::Open, &rows, 1), Effect::SetOpen(1, true));
        assert_eq!(apply_input(TreeInput::Open, &rows, 0), Effect::Cursor(1)); // already open: go in
        assert_eq!(apply_input(TreeInput::Open, &rows, 2), Effect::None); // a leaf
        assert_eq!(apply_input(TreeInput::Close, &rows, 0), Effect::SetOpen(0, false));
        assert_eq!(apply_input(TreeInput::Close, &rows, 2), Effect::Cursor(0)); // to the parent

        assert_eq!(apply_input(TreeInput::Next, &[], 0), Effect::Cursor(0)); // nothing to show
        assert_eq!(apply_input(TreeInput::Close, &[], 0), Effect::None);
    }
}
