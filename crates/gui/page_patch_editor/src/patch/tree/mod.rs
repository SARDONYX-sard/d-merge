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

use super::workspace::{Behavior, ObjectText};

mod state;
#[cfg(test)]
mod tests;

pub(super) use state::{MAX_STICKY, ObjectTree, Row, TreeKey, TreeState};

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
