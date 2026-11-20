//! The whole graph as a tree ([`crate::LayoutOptions::all`]).
//!
//! Each subtree is in a band of its own, its parent in the middle of its children, a gap between
//! subtrees. A big file reads by its groups then (a state machine and its states), not as columns
//! of hundreds of nodes.
//!
//! ```text
//!                          ┌─ [Idle] ─┐
//!               ┌─[SM]─────┼─ [Walk]  ├──> [Clip]   <- shared by Idle and Run: after their
//! [Root]────────┤          └─ [Run] ──┘                subtrees, right of both
//!               │
//!               └─[Graph]──── [Clip2]     <- a gap between the subtrees
//! ```
//!
//! The tree is the walk's ([`crate::layout::LaidEdge::tree`]), but for a node pointed to from
//! several places: that one goes under the nearest node all its parents are under (their lowest
//! common ancestor), right after the subtree of the last of them. It is then near all of them, and
//! their edges are short, not across the graph. (Its column, right of all of them, is the layout's.)
//!
//! Its leaves are stacked from the top, one after another; a parent is centered on its first and
//! last child. Every node has the same height (a card), so no parent can overlap its neighbor.

use crate::layout::Layout;

/// The tree of a whole graph's [`Layout`], by the nodes' indices in it.
#[derive(Debug, Clone, Default)]
pub struct Tree {
    /// In the order of the edges: e.g. `states[0]`, `states[1]`, ...
    children: Vec<Vec<usize>>,
    /// Of the tree: not always a parent in the graph (see the module docs).
    parent: Vec<Option<usize>>,
    /// The nodes without a parent, in the layout's order.
    roots: Vec<usize>,
}

impl Tree {
    /// The tree of `layout`: see the module docs.
    #[must_use]
    pub fn of<N>(layout: &Layout<N>) -> Self {
        let len = layout.nodes.len();
        let mut tree = Self { children: vec![Vec::new(); len], parent: vec![None; len], roots: Vec::new() };
        let mut parents: Vec<Vec<usize>> = vec![Vec::new(); len];
        for edge in layout.edges.iter().filter(|edge| edge.from != edge.to) {
            if edge.tree && tree.parent[edge.to].is_none() {
                tree.parent[edge.to] = Some(edge.from);
                tree.children[edge.from].push(edge.to);
            }
            if !parents[edge.to].contains(&edge.from) {
                parents[edge.to].push(edge.from);
            }
        }
        for (node, parents) in parents.iter().enumerate().filter(|(_, parents)| parents.len() > 1) {
            tree.move_shared(node, parents);
        }
        tree.roots = (0..len).filter(|&i| tree.parent[i].is_none()).collect();
        tree
    }

    /// Moves `node`, pointed to by `parents`, under their lowest common ancestor: see the module
    /// docs. A parent under `node` (a cycle) does not count.
    fn move_shared(&mut self, node: usize, parents: &[usize]) {
        let above: Vec<usize> = parents.iter().copied().filter(|&parent| !self.is_under(parent, node)).collect();
        let Some(common) = above.iter().copied().reduce(|a, b| self.common_ancestor(a, b).unwrap_or(a)) else {
            return;
        };
        if self.parent[node] == Some(common) || common == node || self.is_under(common, node) {
            return;
        }

        if let Some(old) = self.parent[node] {
            self.children[old].retain(|&child| child != node);
        }
        // Right after the last subtree of `common` that holds a parent.
        let holding = |child: usize| above.iter().any(|&parent| parent == child || self.is_under(parent, child));
        let after = self.children[common].iter().rposition(|&child| holding(child)).map_or(0, |at| at + 1);
        self.children[common].insert(after, node);
        self.parent[node] = Some(common);
    }

    /// The parent of `node` in the tree, its parent, ... up to a root.
    fn ancestors(&self, node: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.parent[node], |&above| self.parent[above])
    }

    /// Is `node` under `above` in the tree?
    fn is_under(&self, node: usize, above: usize) -> bool {
        self.ancestors(node).any(|ancestor| ancestor == above)
    }

    /// The nearest node that `a` and `b` are both under, or are. `None`: in different trees.
    fn common_ancestor(&self, a: usize, b: usize) -> Option<usize> {
        let of_a: Vec<usize> = std::iter::once(a).chain(self.ancestors(a)).collect();
        std::iter::once(b).chain(self.ancestors(b)).find(|node| of_a.contains(node))
    }

    /// The children of node `index`.
    #[inline]
    #[must_use]
    pub fn children(&self, index: usize) -> &[usize] {
        &self.children[index]
    }

    /// The parent of node `index` in the tree: an edge from it is drawn as the tree's.
    #[inline]
    #[must_use]
    pub fn parent(&self, index: usize) -> Option<usize> {
        self.parent[index]
    }

    /// The vertical center of each node, from `0` at the top: `height` tall, `gap` apart, and
    /// `group_gap` more after each subtree.
    ///
    /// e.g. a root with 2 leaves, height 10, gap 4, group gap 20: the leaves at 5 and 19, the root
    /// at 12.
    #[must_use]
    pub fn arrange(&self, height: f32, (gap, group_gap): (f32, f32)) -> Vec<f32> {
        let centers = vec![0.0; self.children.len()];
        let mut stack = Stacking { height, gap, group_gap, cursor: 0.0, after_leaf: 0.0, centers };
        for &root in &self.roots {
            stack.place(self, root);
        }
        stack.centers
    }
}

/// The state of [`Tree::arrange`].
struct Stacking {
    height: f32,
    gap: f32,
    group_gap: f32,
    /// The top of the next leaf.
    cursor: f32,
    /// [`Self::cursor`] right after the last leaf: the subtrees ending there leave one gap after it,
    /// not one each.
    after_leaf: f32,
    centers: Vec<f32>,
}

impl Stacking {
    /// Places node `index` and its subtree. Returns its center.
    fn place(&mut self, tree: &Tree, index: usize) -> f32 {
        let children = tree.children(index);
        let center = if let [first, .., last] = children {
            let top = self.place(tree, *first);
            for &child in &children[1..children.len() - 1] {
                self.place(tree, child);
            }
            let bottom = self.place(tree, *last);
            self.end_subtree();
            (top + bottom) / 2.0
        } else if let [only] = children {
            let center = self.place(tree, *only);
            self.end_subtree();
            center
        } else {
            let center = self.cursor + self.height / 2.0;
            self.cursor += self.height + self.gap;
            self.after_leaf = self.cursor;
            center
        };
        self.centers[index] = center;
        center
    }

    /// A subtree ended: the group gap after its last leaf, once however many subtrees end there.
    const fn end_subtree(&mut self) {
        self.cursor = self.cursor.max(self.after_leaf + self.group_gap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Card, Edge, GraphSource,
        layout::{LayoutOptions, layout},
    };

    /// Nodes `0..n`, edges `(from, to)`.
    struct Mock(Vec<(u32, u32)>);

    impl GraphSource for Mock {
        type Node = u32;

        fn version(&self) -> u64 {
            0
        }

        fn edges(&self, node: u32) -> impl Iterator<Item = Edge<'_, u32>> {
            self.0.iter().filter(move |e| e.0 == node).map(|&(_, to)| Edge { to, label: "", key: to as usize })
        }

        fn parents(&self, node: u32) -> impl Iterator<Item = u32> {
            self.0.iter().filter(move |e| e.1 == node).map(|e| e.0)
        }

        fn nodes(&self) -> impl Iterator<Item = u32> {
            0..10
        }

        fn card(&self, _: u32) -> Card<'_> {
            Card { title: "", subtitle: "", header: None, accent: None, dimmed: false, editable: true, fan_in: 0 }
        }
    }

    fn whole(edges: Vec<(u32, u32)>, focus: u32) -> (Tree, Layout<u32>) {
        let options = LayoutOptions { all: true, ..LayoutOptions::DEFAULT };
        let layout = layout(&Mock(edges), focus, options, &|_, _| true);
        (Tree::of(&layout), layout)
    }

    fn index(layout: &Layout<u32>, node: u32) -> usize {
        layout.index_of(node).unwrap()
    }

    /// The walk's edges make the tree; a node of several parents goes under their lowest common
    /// ancestor, right after the subtree of the last of them.
    #[test]
    fn a_shared_node_goes_under_its_parents_common_ancestor() {
        // 0 -> (1, 4, 7); 1 -> 2 -> 3; 4 -> 3. The walk finds 3 from 4; 0 holds both 2 and 4.
        let (tree, layout) = whole(vec![(0, 1), (0, 4), (0, 7), (1, 2), (2, 3), (4, 3)], 0);
        let at = |node| index(&layout, node);
        assert_eq!(tree.children(at(0)), [at(1), at(4), at(3), at(7)], "after 4's subtree, before 7");
        assert!(tree.children(at(4)).is_empty(), "moved from where the walk found it");
        assert_eq!(tree.parent(at(3)), Some(at(0)));
    }

    /// A parent under the shared node (a cycle) does not count: it stays where it was.
    #[test]
    fn a_cycle_does_not_move_a_node() {
        let (tree, layout) = whole(vec![(0, 1), (1, 2), (2, 1)], 0);
        assert_eq!(tree.parent(index(&layout, 1)), Some(index(&layout, 0)));
    }

    /// Where subtrees of several depths end together, one gap: not one each.
    #[test]
    fn subtrees_ending_together_leave_one_gap() {
        // 6 -> 7 -> 8 -> (9, 5), and 6 -> 4: between 5 and 4, the subtrees of 8 and 7 end.
        let source = Mock(vec![(6, 7), (7, 8), (8, 9), (8, 5), (6, 4)]);
        let layout = layout(&source, 6, LayoutOptions { all: true, ..LayoutOptions::DEFAULT }, &|_, _| true);
        let centers = Tree::of(&layout).arrange(10.0, (4.0, 20.0));
        let center = |node| centers[index(&layout, node)];
        assert!((center(4) - center(5) - (10.0 + 4.0 + 20.0)).abs() < f32::EPSILON, "{} {}", center(5), center(4));
    }

    /// Leaves stacked from the top, parents centered on their children, a gap after each subtree.
    #[test]
    fn subtrees_are_bands() {
        let (tree, layout) = whole(vec![(0, 1), (1, 2), (1, 3), (0, 4)], 0);
        let centers = tree.arrange(10.0, (4.0, 20.0));
        let center = |node| centers[index(&layout, node)];
        assert_eq!((center(2), center(3)), (5.0, 19.0), "2 and 3, one after the other");
        assert!((center(1) - 12.0).abs() < f32::EPSILON, "in the middle of its children");
        assert!(center(4) > center(3) + 20.0, "after the gap of 1's subtree: {}", center(4));
    }
}
