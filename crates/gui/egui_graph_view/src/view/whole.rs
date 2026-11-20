//! The whole graph ([`crate::LayoutOptions::all`]) as a tree: see [`crate::tree`].
//!
//! It is for seeing how everything connects, so every node is shown, nothing folded, and closer
//! together than a neighborhood (half its gaps): a file has thousands of nodes.

use super::{GraphView, Placement, metrics};

/// The room after each subtree, at zoom 1, on top of the gap between nodes.
const GROUP_GAP: f32 = 10.0;
/// The whole graph's gaps, as a share of the neighborhood's ([`GraphView::gap`]).
const GAP_SHARE: f32 = 0.5;

impl<N: Copy + Eq + std::hash::Hash + std::fmt::Debug> GraphView<N> {
    /// The space between columns (`x`) and nodes (`y`) at zoom 1: the whole graph's are closer.
    pub(super) fn gaps(&self, placement: &Placement<N>) -> egui::Vec2 {
        if placement.tree.is_some() { self.gap * GAP_SHARE } else { self.gap }
    }

    /// Places the whole graph's nodes by the tree, once per layout. Does nothing for a neighborhood.
    pub(super) fn arrange_whole(&self, placement: &mut Placement<N>) {
        let Some(tree) = placement.tree.as_ref().filter(|_| !placement.arranged) else { return };
        let gap = self.gaps(placement).y;
        placement.centers = tree.arrange(metrics::HEADER, (gap, GROUP_GAP));
        // The layout's layers: each node right of all that point to it (a shared one too).
        placement.columns = placement.layout.nodes.iter().map(|placed| placed.layer).collect();
        placement.arranged = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Card, Edge, GraphSource, LayoutOptions};

    /// A chain `0 -> 1 -> 2 -> 3 -> 4`.
    struct Chain;

    impl GraphSource for Chain {
        type Node = u32;

        fn version(&self) -> u64 {
            0
        }

        fn edges(&self, node: u32) -> impl Iterator<Item = Edge<'_, u32>> {
            (node < 4).then_some(Edge { to: node + 1, label: "", key: 0 }).into_iter()
        }

        fn parents(&self, node: u32) -> impl Iterator<Item = u32> {
            node.checked_sub(1).into_iter()
        }

        fn nodes(&self) -> impl Iterator<Item = u32> {
            0..5
        }

        fn card(&self, _: u32) -> Card<'_> {
            Card { title: "", subtitle: "", header: None, accent: None, dimmed: false, editable: true, fan_in: 0 }
        }
    }

    /// Every node shown, each a column deeper than its parent, however deep; half the gaps.
    #[test]
    fn every_node_is_shown_by_its_depth() {
        let mut view = GraphView::new().options(LayoutOptions { all: true, ..LayoutOptions::DEFAULT });
        view.refresh(&Chain, 0);
        let mut placement = view.placement.take().unwrap();
        view.arrange_whole(&mut placement);
        let columns: Vec<(u32, i32)> = placement
            .layout
            .nodes
            .iter()
            .zip(&placement.columns)
            .map(|(placed, &column)| (placed.node, column))
            .collect();
        assert_eq!(columns, [(0, 0), (1, 1), (2, 2), (3, 3), (4, 4)]);
        assert_eq!(view.gaps(&placement), GraphView::<u32>::GAP / 2.0);
    }
}
