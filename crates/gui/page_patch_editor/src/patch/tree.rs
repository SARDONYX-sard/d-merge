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
//! under the first parent a depth-first walk meets ([`Entry::first_parent`]); elsewhere it is a
//! dimmed leaf.
//!
//! # What is recomputed when
//! A behavior has thousands of objects, its text changes on every key press, and the tree is drawn
//! every frame. So each layer only does the work its input change requires:
//!
//! | Change                      | Work                                                       |
//! | --------------------------- | ---------------------------------------------------------- |
//! | a frame                     | draws the rows on screen                                   |
//! | typing, pointers unchanged  | re-reads that one object's text ([`Graph::update`])        |
//! | a pointer added / removed   | also a walk over the node numbers (no text): [`Graph::finish_updates`] |
//! | a node opened / closed      | rebuilds the flat rows ([`TreeState::rows`])               |
//!
//! The walk does not depend on what is expanded, so neither does the "not linked" count.

use std::{
    collections::BTreeMap,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use fast_hash::{HashMap, HashSet};

use super::{
    tool::list::{ListScroll, offset_to_show},
    workspace::{Behavior, ObjectText},
};

/// Index of an object in [`Graph`]. Stable for the life of the graph, also across edits.
pub(super) type Node = u32;

/// A number no earlier state of any graph had, so that a cache can tell "is this still what I was
/// built from?", also after switching to another behavior's graph.
fn next_generation() -> u64 {
    static LAST: AtomicU64 = AtomicU64::new(0);
    LAST.fetch_add(1, Ordering::Relaxed) + 1
}

/// One id seen in the behavior.
#[derive(Debug, Clone)]
struct Entry {
    id: Rc<str>,
    /// `false`: only a pointer mentions this id (a typo, a deleted object, one not added yet).
    /// Such a node has no text. Keeping it means adding that object later needs no search for who
    /// mentions it.
    exists: bool,
    /// A `#code$N` object of this mod (not in the template).
    is_new: bool,
    /// A template object with edits.
    is_edited: bool,
    /// e.g. `● #0106  hkbModifierList  GetUpModList`, `+ #mymod$0  hkbClipGenerator   ← #0106`
    label: String,
    /// Where the text after the id starts in `label` (`hkbModifierList  GetUpModList`).
    doc_start: u32,
    /// Where the object's name starts in `label`.
    name_start: u32,
    /// Where the `← #0106` part of a new object starts in `label`.
    links_start: u32,
    /// The nodes this object's text mentions, first occurrence order.
    children: Vec<Node>,
    /// The objects mentioning this id, sorted.
    parents: Vec<Node>,
    /// The parent it expands under (`None`: the root, or not reachable from it).
    first_parent: Option<Node>,
    /// Reachable from the root.
    reachable: bool,
    /// Changed, or (transitively) points to a changed object: the "changed only" tree.
    relevant: bool,
}

impl Entry {
    fn new(id: &str) -> Self {
        Self {
            id: Rc::from(id),
            exists: false,
            is_new: false,
            is_edited: false,
            label: String::new(),
            doc_start: 0,
            name_start: 0,
            links_start: 0,
            children: Vec::new(),
            parents: Vec::new(),
            first_parent: None,
            reachable: false,
            relevant: false,
        }
    }

    const fn is_changed(&self) -> bool {
        self.is_new || self.is_edited
    }
}

/// The pointers between all objects of a behavior. See the module docs for what updates when.
#[derive(Debug, Clone, Default)]
pub(super) struct Graph {
    entries: Vec<Entry>,
    index: HashMap<Rc<str>, Node>,
    /// New objects by id, for the list order and the "not linked" check.
    new_nodes: BTreeMap<Rc<str>, Node>,
    /// Template objects in file order.
    template_nodes: Vec<Node>,
    root: Option<Node>,
    /// New objects the root does not reach: Nemesis would never use them.
    unlinked: Vec<Node>,
    /// A pointer or a "changed" state differs from the last [`Self::finish_updates`].
    links_dirty: bool,
    /// Changes with anything, e.g. a label.
    generation: u64,
    /// Changes when the shape of the tree does: links, "changed" states, which objects exist.
    tree_generation: u64,
}

impl Graph {
    pub(super) fn new(behavior: &Behavior) -> Self {
        let objects: Vec<_> = behavior.texts().collect();
        let root = behavior
            .object_of_class("hkRootLevelContainer")
            .or_else(|| behavior.objects.first().map(|o| o.id.as_str()));
        Self::from_objects(&objects, root)
    }

    fn from_objects(objects: &[ObjectText<'_>], root: Option<&str>) -> Self {
        let mut graph = Self::default();
        for object in objects {
            graph.update(object.id, Some(*object));
        }
        graph.root = root.and_then(|id| graph.node_of(id));
        graph.finish_updates();
        graph
    }

    /// Takes in the current state of one object: `None` = it no longer exists.
    /// Call [`Self::finish_updates`] after the last one of a batch.
    pub(super) fn update(&mut self, id: &str, object: Option<ObjectText<'_>>) {
        self.generation = next_generation();
        let node = self.intern(id);
        match object {
            Some(object) => self.set_object(node, object),
            None => self.remove_object(node),
        }
    }

    /// Recomputes what depends on the links, if any changed. Only looks at node numbers, not text.
    pub(super) fn finish_updates(&mut self) {
        if !std::mem::take(&mut self.links_dirty) {
            return;
        }
        self.walk_from_root();
        self.compute_relevant();
        self.unlinked = self
            .new_nodes
            .values()
            .copied()
            .filter(|&node| !self.entry(node).reachable && self.entry(node).id.starts_with('#'))
            .collect();
        self.tree_generation = next_generation();
    }

    /// The node of `id`, adding one if this is the first time it is seen.
    fn intern(&mut self, id: &str) -> Node {
        if let Some(&node) = self.index.get(id) {
            return node;
        }
        let node = node_at(self.entries.len());
        let entry = Entry::new(id);
        self.index.insert(Rc::clone(&entry.id), node);
        self.entries.push(entry);
        node
    }

    fn set_object(&mut self, node: Node, object: ObjectText<'_>) {
        let entry = &mut self.entries[node as usize];
        if !entry.exists || entry.is_edited != object.is_edited {
            self.links_dirty = true;
        }
        let appeared = !entry.exists;
        entry.exists = true;
        entry.is_new = object.is_new;
        entry.is_edited = object.is_edited;
        if appeared {
            if object.is_new {
                self.new_nodes.insert(Rc::clone(&entry.id), node);
            } else {
                self.template_nodes.push(node);
            }
        }

        let children: Vec<Node> = pointers(object.text, object.id).into_iter().map(|id| self.intern(id)).collect();
        self.set_children(node, children);
        self.set_label(node, object);
    }

    /// Only new objects can be deleted. The node stays, in case something still mentions the id.
    fn remove_object(&mut self, node: Node) {
        let entry = &mut self.entries[node as usize];
        if !entry.exists {
            return;
        }
        entry.exists = false;
        entry.is_edited = false;
        entry.label.clear();
        let id = Rc::clone(&entry.id);
        self.new_nodes.remove(&id);
        self.links_dirty = true;
        self.set_children(node, Vec::new());
    }

    /// Replaces `node`'s children and updates the parents of those added and removed.
    fn set_children(&mut self, node: Node, children: Vec<Node>) {
        if self.entry(node).children == children {
            return; // the usual case: typing that does not touch a pointer
        }
        self.links_dirty = true;
        let old = std::mem::replace(&mut self.entries[node as usize].children, children);

        let new_set: HashSet<Node> = self.entry(node).children.iter().copied().collect();
        let old_set: HashSet<Node> = old.iter().copied().collect();
        for &removed in old.iter().filter(|child| !new_set.contains(child)) {
            let parents = &mut self.entries[removed as usize].parents;
            if let Ok(at) = parents.binary_search(&node) {
                parents.remove(at);
            }
            self.refresh_links_text(removed);
        }
        for i in 0..self.entry(node).children.len() {
            let added = self.entry(node).children[i];
            if !old_set.contains(&added) {
                let parents = &mut self.entries[added as usize].parents;
                if let Err(at) = parents.binary_search(&node) {
                    parents.insert(at, node);
                }
                self.refresh_links_text(added);
            }
        }
    }

    fn set_label(&mut self, node: Node, object: ObjectText<'_>) {
        let marker = if object.is_new {
            '+'
        } else if object.is_edited {
            '●'
        } else {
            ' '
        };
        let name = field_name(object.text).unwrap_or(object.name);
        let entry = &mut self.entries[node as usize];
        entry.label.clear();
        entry.label.extend([marker, ' ']);
        entry.label.push_str(object.id);
        entry.label.push_str("  ");
        entry.doc_start = text_offset(entry.label.len());
        entry.label.push_str(object.class);
        entry.label.push_str("  ");
        entry.name_start = text_offset(entry.label.len());
        entry.label.push_str(name);
        entry.links_start = text_offset(entry.label.len());
        self.refresh_links_text(node);
    }

    /// The end of a new object's label: where it is linked, e.g. ` ← #0106, #0107`.
    fn refresh_links_text(&mut self, node: Node) {
        let entry = self.entry(node);
        // (Clip data files are "new" too, but nothing points to them: they have no `#` id.)
        if !entry.exists || !entry.is_new || !entry.id.starts_with('#') {
            return;
        }
        let mut links = String::new();
        for (i, &parent) in entry.parents.iter().enumerate() {
            links.push_str(if i == 0 { " ← " } else { ", " });
            links.push_str(&self.entry(parent).id);
        }
        if links.is_empty() {
            links.push_str(" ⚠ not linked");
        }
        let entry = &mut self.entries[node as usize];
        entry.label.truncate(entry.links_start as usize);
        entry.label.push_str(&links);
    }

    /// Depth-first from the root: what it reaches, and from which parent first.
    fn walk_from_root(&mut self) {
        fn visit(graph: &mut Graph, node: Node) {
            graph.entries[node as usize].reachable = true;
            for i in 0..graph.entry(node).children.len() {
                let child = graph.entry(node).children[i];
                let entry = graph.entry(child);
                if entry.exists && !entry.reachable {
                    graph.entries[child as usize].first_parent = Some(node);
                    visit(graph, child);
                }
            }
        }
        for entry in &mut self.entries {
            entry.reachable = false;
            entry.first_parent = None;
        }
        if let Some(root) = self.root {
            visit(self, root);
        }
    }

    fn compute_relevant(&mut self) {
        fn visit(graph: &mut Graph, node: Node, memo: &mut [Option<bool>]) -> bool {
            if let Some(known) = memo[node as usize] {
                return known;
            }
            memo[node as usize] = Some(false); // a cycle back here adds nothing
            let entry = graph.entry(node);
            let mut relevant = entry.exists && entry.is_changed();
            for i in 0..graph.entry(node).children.len() {
                let child = graph.entry(node).children[i];
                relevant |= visit(graph, child, memo);
            }
            memo[node as usize] = Some(relevant);
            graph.entries[node as usize].relevant = relevant;
            relevant
        }
        let mut memo = vec![None; self.entries.len()];
        for i in 0..self.entries.len() {
            visit(self, node_at(i), &mut memo);
        }
    }

    fn entry(&self, node: Node) -> &Entry {
        &self.entries[node as usize]
    }

    pub(super) fn id(&self, node: Node) -> &str {
        &self.entry(node).id
    }

    pub(super) fn label(&self, node: Node) -> &str {
        &self.entry(node).label
    }

    /// A short name for the way to an object: its `name` field, or else its class.
    ///
    /// e.g. `GetUpModList`, `hkbVariableBindingSet`
    pub(super) fn short_name(&self, node: Node) -> &str {
        let (_, class, name, _) = self.label_parts(node);
        if name.trim().is_empty() { class.trim() } else { name.trim() }
    }

    /// e.g. `hkbModifierList  GetUpModList`: the object in hovers and completion.
    pub(super) fn doc(&self, node: Node) -> &str {
        let entry = self.entry(node);
        &entry.label[entry.doc_start as usize..]
    }

    /// The label split for coloring: `(marker and id, class, name, where it is linked)`.
    pub(super) fn label_parts(&self, node: Node) -> (&str, &str, &str, &str) {
        let entry = self.entry(node);
        let label = entry.label.as_str();
        let (doc, name, links) = (entry.doc_start as usize, entry.name_start as usize, entry.links_start as usize);
        (&label[..doc], &label[doc..name], &label[name..links], &label[links..])
    }

    /// The node of an **existing** object.
    pub(super) fn node_of(&self, id: &str) -> Option<Node> {
        self.index.get(id).copied().filter(|&node| self.entry(node).exists)
    }

    /// Every object: the new ones first, then the template's.
    pub(super) fn nodes(&self) -> impl Iterator<Item = Node> {
        self.new_nodes.values().chain(&self.template_nodes).copied()
    }

    pub(super) fn is_changed(&self, node: Node) -> bool {
        self.entry(node).is_changed()
    }

    pub(super) fn is_new(&self, node: Node) -> bool {
        self.entry(node).is_new
    }

    pub(super) fn unlinked(&self) -> &[Node] {
        &self.unlinked
    }

    /// Is `node` one of [`Self::unlinked`]? Without looking through them.
    pub(super) fn is_unlinked(&self, node: Node) -> bool {
        let entry = self.entry(node);
        entry.is_new && !entry.reachable && entry.id.starts_with('#')
    }

    pub(super) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) const fn tree_generation(&self) -> u64 {
        self.tree_generation
    }

    /// `node`'s children in the tree being shown.
    fn shown_children(&self, node: Node, changed_only: bool) -> impl Iterator<Item = Node> {
        self.entry(node).children.iter().copied().filter(move |&child| {
            let entry = self.entry(child);
            entry.exists && (!changed_only || entry.relevant)
        })
    }
}

/// `usize` -> [`Node`]. (A behavior has far fewer than 4 billion objects.)
#[expect(clippy::cast_possible_truncation, reason = "see above")]
const fn node_at(index: usize) -> Node {
    index as Node
}

/// A byte offset in a label. (A label is far shorter than 4 GiB.)
#[expect(clippy::cast_possible_truncation, reason = "see above")]
const fn text_offset(len: usize) -> u32 {
    len as u32
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
    /// The last child of its parent: its connecting line ends here (`└`, not `├`).
    pub(super) last_child: bool,
    /// The row of its parent. `None`: the root.
    pub(super) parent_row: Option<u32>,
    /// Bit `n`: the line of tree level `n` continues past this row, as that ancestor has more
    /// children below. e.g., `│   └ row` has bit 0.
    pub(super) guides: u64,
}

/// How a row connects to the rows around it. See [`Row`].
#[derive(Debug, Clone, Copy)]
struct RowLines {
    parent_row: Option<u32>,
    last_child: bool,
    guides: u64,
}

/// The bit of tree level `depth` in [`Row::guides`]. Deeper than 64 levels has no lines.
const fn guide_bit(depth: u16) -> u64 {
    if depth < u64::BITS as u16 { 1 << depth } else { 0 }
}

/// Does the line of tree level `level` continue past the row with these `guides`?
pub(super) const fn has_guide(guides: u64, level: u16) -> bool {
    guides & guide_bit(level) != 0
}

/// How many ancestors are pinned over the tree at most. See [`TreeState::sticky_rows`].
pub(super) const MAX_STICKY: usize = 5;

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
    /// `o`: open the node, or close it if it is open. The cursor stays.
    Toggle,
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
            Self::Close | Self::Open | Self::Toggle | Self::Leave => None,
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
        TreeInput::Toggle => match row {
            Some(row) if row.expandable => Effect::SetOpen(row.node, !row.open),
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
    /// `open` changed since the settings last took it. See [`Self::take_open_changes`].
    open_changed: bool,
    rows: Vec<Row>,
    /// What `rows` was built from: [`Graph::tree_generation`] and "changed only".
    built_for: Option<(u64, bool)>,
    /// The row of the selected object.
    pub(super) cursor: usize,
    /// The editor got the keyboard: the next [`Self::prepare`] opens the way to the selection and
    /// scrolls to it. Only then: a tree being used must not move by itself.
    reveal_asked: bool,
    /// The selection the cursor was put on (it follows a selection made elsewhere, without scrolling).
    cursor_for: Option<Rc<str>>,
    /// Bring this row into view on the next draw.
    pub(super) scroll_to: Option<usize>,
    pub(super) scroll_offset: f32,
    pub(super) view_height: f32,
    pub(super) scroll_x: f32,
    pub(super) view_width: f32,
    /// `Space` was pressed: the next `e` leaves the tree (same keys as entering it).
    pub(super) leader: bool,
}

impl TreeState {
    /// A tree with the open / closed nodes saved in the settings.
    pub(super) fn restored(open: &BTreeMap<String, bool>) -> Self {
        let open = open.iter().map(|(id, open)| (Rc::from(id.as_str()), *open)).collect();
        Self { open, ..Self::default() }
    }

    /// The open / closed nodes to save, if they changed since the last call.
    pub(super) fn take_open_changes(&mut self) -> Option<BTreeMap<String, bool>> {
        std::mem::take(&mut self.open_changed)
            .then(|| self.open.iter().map(|(id, open)| (id.to_string(), *open)).collect())
    }

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
        self.open.insert(Rc::clone(&graph.entry(node).id), open);
        self.open_changed = true;
        self.built_for = None;
    }

    /// Opens / closes `node` and everything expanding under it.
    pub(super) fn set_open_below(&mut self, graph: &Graph, node: Node, open: bool) {
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            self.set_open(graph, current, open);
            stack.extend(
                graph.entry(current).children.iter().filter(|&&c| graph.entry(c).first_parent == Some(current)),
            );
        }
    }

    /// Rebuilds the rows if needed, and puts the cursor on `selected`. Opens the way to it and scrolls
    /// there only if [`Self::reveal_again`] asked for it.
    ///
    /// ```text
    /// selection changed (a click in the tree, a tab, `gd`)  -> the cursor follows; nothing moves
    /// the editor got the keyboard (reveal_again)          -> parents opened, scrolled to it
    /// ```
    pub(super) fn prepare(&mut self, graph: &Graph, selected: Option<&str>, changed_only: bool) {
        let revealed = std::mem::take(&mut self.reveal_asked) && self.reveal(graph, selected, changed_only);
        let stale = self.built_for != Some((graph.tree_generation(), changed_only));
        if stale {
            self.rebuild_rows(graph, changed_only);
        }
        let moved = self.cursor_for.as_deref() != selected;
        if revealed || stale || moved {
            self.cursor_for = selected.map(Rc::from);
            self.follow_selection(graph, selected, revealed);
        }
    }

    /// The editor got the keyboard: the next [`Self::prepare`] shows the selection (opens its parents,
    /// scrolls to it), as VS Code's explorer follows the file being edited.
    pub(super) const fn reveal_again(&mut self) {
        self.reveal_asked = true;
    }

    /// Opens the parents of `selected`. Returns whether it is in the tree.
    fn reveal(&mut self, graph: &Graph, selected: Option<&str>, changed_only: bool) -> bool {
        let Some(target) = selected.and_then(|id| graph.node_of(id)) else { return false };
        let mut parent = graph.entry(target).first_parent;
        while let Some(node) = parent {
            if !self.is_open(graph, node, changed_only) {
                self.set_open(graph, node, true);
            }
            parent = graph.entry(node).first_parent;
        }
        true
    }

    fn rebuild_rows(&mut self, graph: &Graph, changed_only: bool) {
        let mut rows = std::mem::take(&mut self.rows);
        rows.clear();
        if let Some(root) = graph.root {
            let lines = RowLines { parent_row: None, last_child: true, guides: 0 };
            self.push_rows(graph, root, 0, lines, changed_only, &mut rows);
        }
        self.rows = rows;
        self.built_for = Some((graph.tree_generation(), changed_only));
    }

    fn push_rows(
        &self,
        graph: &Graph,
        node: Node,
        depth: u16,
        lines: RowLines,
        changed_only: bool,
        rows: &mut Vec<Row>,
    ) {
        let expandable = graph.shown_children(node, changed_only).next().is_some();
        let open = expandable && self.is_open(graph, node, changed_only);
        let RowLines { parent_row, last_child, guides } = lines;
        #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion rows")]
        let this_row = Some(rows.len() as u32);
        rows.push(Row { node, depth, expandable, open, dimmed: false, last_child, parent_row, guides });
        if !open {
            return;
        }
        let children: Vec<Node> = graph.shown_children(node, changed_only).collect();
        for (i, &child) in children.iter().enumerate() {
            let last_child = i + 1 == children.len();
            // This node's children hang from a line at this depth: more of them keep it going.
            let guides = if last_child { guides } else { guides | guide_bit(depth) };
            if graph.entry(child).first_parent == Some(node) {
                let lines = RowLines { parent_row: this_row, last_child, guides };
                self.push_rows(graph, child, depth + 1, lines, changed_only, rows);
            } else {
                let (expandable, open, dimmed) = (false, false, true);
                let (depth, parent_row) = (depth + 1, this_row);
                rows.push(Row { node: child, depth, expandable, open, dimmed, last_child, parent_row, guides });
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
        graph: &Graph,
        selected: &mut Option<Rc<str>>,
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
    fn select_cursor(&mut self, graph: &Graph, selected: &mut Option<Rc<str>>) {
        self.scroll_to = Some(self.cursor);
        if let Some(row) = self.rows.get(self.cursor) {
            self.cursor_for = Some(Rc::clone(&graph.entry(row.node).id));
            *selected = Some(graph.id(row.node).into());
        }
    }

    /// The rows of `row`'s ancestors, root first: the way from the root to it.
    ///
    /// e.g., for `#c` in `#root > #b > #c`: the rows of `#root` and `#b`.
    pub(super) fn ancestors(&self, row: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut current = self.rows.get(row).and_then(|r| r.parent_row);
        while let Some(parent) = current {
            chain.push(parent as usize);
            current = self.rows[parent as usize].parent_row;
        }
        chain.reverse();
        chain
    }

    /// The rows to pin at the top of the view (like VS Code's sticky scroll): the ancestors of what
    /// is shown under them, or at most the `max` nearest, so you see which branch you are in after
    /// scrolling far from its parents. `step`: a row's height with its spacing.
    pub(super) fn sticky_rows(&self, step: f32, max: usize) -> Vec<usize> {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a positive row number")]
        let top = (self.scroll_offset / step).max(0.0) as usize;
        // The pinned rows cover as many rows as they are: go by the first row still seen.
        let covered = self.ancestors(top).len().min(max);
        let chain = self.ancestors((top + covered).min(self.rows.len().saturating_sub(1)));
        chain[chain.len().saturating_sub(max)..].to_vec()
    }

    /// The way to the selected row on the connecting lines: per tree level, the rows whose line at
    /// that level is part of it. e.g., `[Some(0..=3), Some(1..=2)]`.
    pub(super) fn active_guides(&self) -> Vec<Option<std::ops::RangeInclusive<usize>>> {
        let mut levels = Vec::new();
        let mut child = self.cursor;
        for parent in self.ancestors(self.cursor).into_iter().rev() {
            let level = usize::from(self.rows[parent].depth);
            if levels.len() <= level {
                levels.resize(level + 1, None);
            }
            levels[level] = Some(parent + 1..=child);
            child = parent;
        }
        levels
    }

    /// The horizontal offset that shows the start of a row indented by `indent` pixels (`None`: it
    /// is on screen). Deep rows would otherwise be out of view to the right.
    pub(super) fn offset_x_for(&self, indent: f32) -> Option<f32> {
        const MARGIN: f32 = 24.0;
        let visible = self.scroll_x..=self.scroll_x + self.view_width / 2.0;
        (!visible.contains(&indent)).then_some((indent - MARGIN).max(0.0))
    }

    /// The scroll offset that brings row `row` into view, below its parents pinned over the top
    /// ([`Self::sticky_rows`]) (`None`: it is seen already). See [`offset_to_show`].
    pub(super) fn offset_for(&self, row: usize, sizes: (f32, f32)) -> Option<f32> {
        let covered = self.ancestors(row).len().min(MAX_STICKY);
        let scroll = ListScroll { offset: self.scroll_offset, view: self.view_height };
        offset_to_show(row, sizes, self.rows.len(), scroll, covered)
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

    /// How long the walk over all the nodes takes on the largest template: what a pointer edit costs.
    #[test]
    #[ignore = "needs the template files of the repository; prints timings"]
    fn time_the_walk_on_the_largest_template() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../resource/assets/templates/meshes/actors/character/behaviors/0_master.bin");
        let behavior = Behavior::open(&path).unwrap();

        let start = std::time::Instant::now();
        let mut graph = Graph::new(&behavior);
        println!("{} nodes, whole build: {:?}", graph.entries.len(), start.elapsed());

        const RUNS: u32 = 100;
        let start = std::time::Instant::now();
        for _ in 0..RUNS {
            graph.links_dirty = true;
            graph.finish_updates();
        }
        println!("walk: {:?} each", start.elapsed() / RUNS);
    }

    fn template<'a>(id: &'a str, text: &'a str) -> ObjectText<'a> {
        ObjectText { id, class: "c", name: "", text, is_new: false, is_edited: false }
    }

    fn new_object<'a>(id: &'a str, text: &'a str) -> ObjectText<'a> {
        ObjectText { id, class: "c", name: "", text, is_new: true, is_edited: false }
    }

    /// `#new0` is reached through `#b`, `#new1` through nothing. `#c` is shared by `#a` and `#b`.
    fn sample() -> Graph {
        let objects = [
            new_object("#new0", ""),
            new_object("#new1", ""),
            template("#root", "#a #b"),
            template("#a", "#c"),
            template("#b", "#c #new0"),
            template("#c", ""),
        ];
        Graph::from_objects(&objects, Some("#root"))
    }

    fn ids<'a>(graph: &'a Graph, state: &TreeState) -> Vec<&'a str> {
        state.rows().iter().map(|row| graph.id(row.node)).collect()
    }

    fn unlinked(graph: &Graph) -> Vec<&str> {
        graph.unlinked().iter().map(|&node| graph.id(node)).collect()
    }

    /// Everything open: the whole tree as `(id, dimmed)`.
    fn whole_tree(graph: &Graph) -> Vec<(String, bool)> {
        let mut state = TreeState::default();
        state.prepare(graph, None, false);
        if let Some(root) = graph.root {
            state.set_open_below(graph, root, true);
        }
        state.prepare(graph, None, false);
        state.rows().iter().map(|row| (graph.id(row.node).to_owned(), row.dimmed)).collect()
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
        assert_eq!(unlinked(&graph), ["#new1"]);

        let mut state = TreeState::default();
        state.prepare(&graph, None, false);
        assert_eq!(ids(&graph, &state), ["#root", "#a", "#b"]); // only the root starts open
        assert_eq!(unlinked(&graph), ["#new1"]);
    }

    /// The list's "unlinked only" asks each node: it must agree with the list of them.
    #[test]
    fn is_unlinked_agrees_with_the_unlinked_list() {
        let graph = sample();
        let asked: Vec<&str> =
            graph.nodes().filter(|&node| graph.is_unlinked(node)).map(|node| graph.id(node)).collect();
        assert_eq!(asked, unlinked(&graph));
    }

    #[test]
    fn labels_tell_where_new_objects_are_linked() {
        let graph = sample();
        let new0 = graph.node_of("#new0").unwrap();
        assert_eq!(graph.label(new0), "+ #new0  c   ← #b");
        assert_eq!(graph.doc(new0), "c   ← #b");
        assert_eq!(graph.label(graph.node_of("#new1").unwrap()), "+ #new1  c   ⚠ not linked");
    }

    /// `#c` has two parents: it expands under the first one and is dimmed under the other.
    #[test]
    fn shared_objects_expand_once() {
        let tree = whole_tree(&sample());
        let expected = [("#root", false), ("#a", false), ("#c", false), ("#b", false), ("#c", true), ("#new0", false)];
        assert_eq!(tree, expected.map(|(id, dimmed)| (id.to_owned(), dimmed)));
    }

    /// Updating one object at a time must give what building from scratch gives.
    #[test]
    fn updates_match_a_full_build() {
        let mut graph = sample();

        // Link the unlinked object.
        graph.update("#a", Some(template("#a", "#c #new1")));
        graph.finish_updates();
        assert!(unlinked(&graph).is_empty());
        assert_eq!(graph.label(graph.node_of("#new1").unwrap()), "+ #new1  c   ← #a");

        // Add one that was mentioned before it existed.
        graph.update("#c", Some(template("#c", "#new2")));
        graph.update("#new2", Some(new_object("#new2", "")));
        graph.finish_updates();
        assert_eq!(graph.label(graph.node_of("#new2").unwrap()), "+ #new2  c   ← #c");

        // Delete one: it leaves the list and the tree.
        graph.update("#new0", None);
        graph.finish_updates();
        assert!(graph.node_of("#new0").is_none());

        let fresh = Graph::from_objects(
            &[
                new_object("#new1", ""),
                new_object("#new2", ""),
                template("#root", "#a #b"),
                template("#a", "#c #new1"),
                template("#b", "#c #new0"),
                template("#c", "#new2"),
            ],
            Some("#root"),
        );
        assert_eq!(whole_tree(&graph), whole_tree(&fresh));
        assert_eq!(unlinked(&graph), unlinked(&fresh));
        let ids_of = |graph: &Graph| graph.nodes().map(|node| graph.id(node).to_owned()).collect::<Vec<_>>();
        assert_eq!(ids_of(&graph)[..2], ["#new1", "#new2"]);
        assert_eq!(ids_of(&graph).len(), ids_of(&fresh).len());
    }

    /// Typing that leaves the pointers alone must not cost a walk or a rebuild of the rows.
    #[test]
    fn typing_keeps_the_tree_generation() {
        let mut graph = sample();
        let tree_generation = graph.tree_generation();
        let generation = graph.generation();

        graph.update("#a", Some(template("#a", "#c <hkparam name=\"name\">x</hkparam>")));
        graph.finish_updates();

        assert_eq!(graph.tree_generation(), tree_generation);
        assert_ne!(graph.generation(), generation);
        assert_eq!(graph.label(graph.node_of("#a").unwrap()), "  #a  c  x");

        // The first edit of an object does change the tree: it joins "changed only".
        let edited = ObjectText { is_edited: true, ..template("#a", "#c") };
        graph.update("#a", Some(edited));
        graph.finish_updates();
        assert_ne!(graph.tree_generation(), tree_generation);
    }

    #[test]
    fn changed_only_shows_the_way_to_changes() {
        let mut graph = sample();
        let edited = ObjectText { is_edited: true, ..template("#c", "") };
        graph.update("#c", Some(edited));
        graph.finish_updates();
        let mut state = TreeState::default();
        state.prepare(&graph, None, true);
        // `#new0` is new, `#c` edited; `#a`, `#b` lead to them.
        assert_eq!(ids(&graph, &state), ["#root", "#a", "#c", "#b", "#c", "#new0"]);
    }

    /// ```text
    /// #root
    /// ├ #a
    /// │ └ #c
    /// └ #b
    ///   ├ #c (dimmed)
    ///   └ #new0
    /// ```
    #[test]
    fn open_nodes_can_be_saved_and_restored() {
        let graph = sample();
        let mut state = TreeState::default();
        assert_eq!(state.take_open_changes(), None);
        state.set_open(&graph, graph.node_of("#b").unwrap(), true);

        let saved = state.take_open_changes().unwrap();
        assert_eq!(state.take_open_changes(), None); // reported once

        let mut restored = TreeState::restored(&saved);
        restored.prepare(&graph, None, false);
        assert_eq!(ids(&graph, &restored), ["#root", "#a", "#b", "#c", "#new0"]);
    }

    #[test]
    fn knows_the_way_to_a_row() {
        let graph = sample();
        let mut state = TreeState::default();
        state.prepare(&graph, None, false);
        state.set_open_below(&graph, graph.node_of("#root").unwrap(), true);
        state.prepare(&graph, None, false);
        // 0 #root, 1 #a, 2 #c, 3 #b, 4 #c (dimmed), 5 #new0
        assert_eq!(state.ancestors(5), [0, 3]);
        assert_eq!(state.ancestors(4), [0, 3]);
        assert!(state.ancestors(0).is_empty());
        assert!(state.ancestors(99).is_empty());

        state.cursor = 5;
        assert_eq!(state.active_guides(), [Some(1..=3), Some(4..=5)]);

        // Scrolled so that `#new0` is the top row: its parents get pinned.
        state.scroll_offset = 50.0;
        assert_eq!(state.sticky_rows(10.0, 5), [0, 3]);
        assert_eq!(state.sticky_rows(10.0, 1), [3]); // only the nearest fit
        state.scroll_offset = 0.0;
        assert_eq!(state.sticky_rows(10.0, 5), Vec::<usize>::new());
    }

    #[test]
    fn horizontal_scroll_only_when_the_indent_is_out_of_view() {
        let state = TreeState { scroll_x: 100.0, view_width: 200.0, ..TreeState::default() };
        assert_eq!(state.offset_x_for(150.0), None);
        assert_eq!(state.offset_x_for(500.0), Some(476.0));
        assert_eq!(state.offset_x_for(10.0), Some(0.0));
    }

    #[test]
    fn rows_know_their_connecting_lines() {
        let graph = sample();
        let mut state = TreeState::default();
        state.prepare(&graph, None, false);
        state.set_open_below(&graph, graph.node_of("#root").unwrap(), true);
        state.prepare(&graph, None, false);
        let rows = state.rows();

        assert_eq!(rows.iter().map(|row| row.last_child).collect::<Vec<_>>(), [true, false, true, true, false, true]);
        // `#c` under `#a`: the line of level 0 goes on to `#b`.
        assert!(has_guide(rows[2].guides, 0));
        // `#c` under `#b`: `#b` is the last child, so level 0 has ended; level 1 goes on to `#new0`.
        assert!(!has_guide(rows[4].guides, 0));
        assert!(has_guide(rows[4].guides, 1));
    }

    /// A selection alone moves nothing: a tree being used must stay as it is.
    #[test]
    fn selecting_alone_does_not_open_or_scroll() {
        let graph = sample();
        let mut state = TreeState::default();
        state.prepare(&graph, Some("#new0"), false);
        assert_eq!(ids(&graph, &state), ["#root", "#a", "#b"], "only the root starts open");
        assert_eq!(state.scroll_to, None);
    }

    /// The editor got the keyboard: the way to the selection opens, and it is scrolled to.
    #[test]
    fn revealing_opens_the_parents_and_scrolls() {
        let graph = sample();
        let mut state = TreeState::default();
        state.reveal_again();
        state.prepare(&graph, Some("#new0"), false);
        assert_eq!(ids(&graph, &state), ["#root", "#a", "#b", "#c", "#new0"]);
        assert_eq!(state.cursor, 4);
        assert_eq!(state.scroll_to, Some(4));
        state.prepare(&graph, Some("#new0"), false);
        assert_eq!(state.scroll_to, Some(4), "asked once: nothing more");
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
        let row = |node, depth, open, expandable| Row {
            node,
            depth,
            expandable,
            open,
            dimmed: false,
            last_child: false,
            parent_row: None,
            guides: 0,
        };
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

        assert_eq!(apply_input(TreeInput::Toggle, &rows, 0), Effect::SetOpen(0, false));
        assert_eq!(apply_input(TreeInput::Toggle, &rows, 1), Effect::SetOpen(1, true));
        assert_eq!(apply_input(TreeInput::Toggle, &rows, 2), Effect::None); // a leaf

        assert_eq!(apply_input(TreeInput::Next, &[], 0), Effect::Cursor(0)); // nothing to show
        assert_eq!(apply_input(TreeInput::Close, &[], 0), Effect::None);
    }
}
