//! The object tree on screen: [`egui_tree::view::TreeView`] over the [`Graph`]. Only what the graph
//! is to the tree is here ([`ObjectTree`]); open nodes, rows, cursor and scrolling are the view's.

pub use egui_tree::TreeKey;
use egui_tree::view::{TreeRow, TreeSource, TreeView};

use super::{Graph, Node};

/// What the tree keeps between frames (open nodes, rows, cursor, scrolling).
pub type TreeState = TreeView<Node>;

/// A line of the tree on screen.
pub type Row = TreeRow<Node>;

/// How many ancestors are pinned over the tree at most, as VS Code's sticky scroll.
pub const MAX_STICKY: usize = 5;

/// The graph as the tree shows it.
///
/// ```text
/// ▼ #0090 hkRootLevelContainer      the root: open at first
///   ▼ #0091 hkbBehaviorGraph        children: the objects it points to
///       #0097 hkbClipGenerator      dimmed: opens under its first parent, elsewhere
/// ```
/// `changed_only`: only the way to edited / new objects, everything open at first.
#[derive(Debug, Clone, Copy)]
pub struct ObjectTree<'a> {
    pub graph: &'a Graph,
    pub changed_only: bool,
}

impl TreeSource for ObjectTree<'_> {
    type Node = Node;

    fn root(&self) -> Option<Node> {
        self.graph.root
    }

    fn children(&self, node: Node) -> impl Iterator<Item = Node> {
        self.graph.shown_children(node, self.changed_only)
    }

    fn parent(&self, node: Node) -> Option<Node> {
        self.graph.entry(node).first_parent
    }

    fn key(&self, node: Node) -> &str {
        self.graph.id(node)
    }

    fn node_of(&self, key: &str) -> Option<Node> {
        self.graph.node_of(key)
    }

    fn default_open(&self, node: Node) -> bool {
        self.graph.root == Some(node) || self.changed_only
    }

    fn version(&self) -> u64 {
        self.graph.tree_generation() * 2 + u64::from(self.changed_only)
    }
}
