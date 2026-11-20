//! Which nodes of the neighborhood are shown, and in which layer and row.
//!
//! ```text
//! 1. find    from the focus (layer 0): its edges give layer 1, theirs layer 2, ...; its parents
//!            give layer -1, theirs -2, ... A full layer hides the rest (`Layer::hidden`). Edges
//!            the caller does not `keep` (e.g. of a page of a list not shown) are not followed.
//!            With `LayoutOptions::all`: every node, from the roots (layer 0), as deep as they go;
//!            what no root reaches (a cycle) from itself, in layer 0 too.
//! 2. edges   every kept edge between two shown nodes. Those the walk went along are the tree's:
//!            the others (to a node met already, e.g. a shared one) can be drawn faintly.
//! 3. deepen  a node pointed to from several places goes right of the deepest of them, so that
//!            every edge goes left to right (a cycle stops after as many steps as there are nodes):
//!
//!              0 ─> 1 ─> 2           0 ─> 1 ─> 2
//!              └───────> 2    not    └─> 2 <─┘   (an edge back to the left, over 1)
//!
//! 4. order   each layer's rows sorted by the mean place of their neighbors in all the layers
//!            before, then after, a few times: fewer crossings (Sugiyama's barycenter method),
//!            also of edges over several layers.
//! ```

use std::collections::{BTreeMap, VecDeque};

use fast_hash::{HashMap, HashSet};

use crate::GraphSource;

/// How much of the neighborhood is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutOptions {
    /// Layers of parents, left of the focus.
    pub depth_up: usize,
    /// Layers of what the focus points to, right of it.
    pub depth_down: usize,
    /// Nodes in a layer: the rest are counted in [`Layer::hidden`].
    pub max_per_layer: usize,
    /// Every node of the source ([`GraphSource::nodes`]), from the roots: not the neighborhood of
    /// the focus. The other options are not used then. The view shows each node's card only, its
    /// rows left out: a layer of a whole file holds hundreds of nodes, stacked.
    pub all: bool,
}

impl LayoutOptions {
    /// Parents, and three layers down: e.g. a state machine, its states, their generators.
    pub const DEFAULT: Self = Self { depth_up: 1, depth_down: 3, max_per_layer: 40, all: false };
}

impl Default for LayoutOptions {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A shown node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed<N> {
    pub node: N,
    /// `0`: the focus; `< 0`: parents; `> 0`: what it points to.
    pub layer: i32,
    /// From the top of its layer.
    pub row: usize,
}

/// An edge between two shown nodes, by their index in [`Layout::nodes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaidEdge {
    pub from: usize,
    pub to: usize,
    pub label: Box<str>,
    /// See [`crate::Edge::key`].
    pub key: usize,
    /// The walk found `to` (or `from`, left of the focus) along it: an edge of the tree that the
    /// others cross.
    pub tree: bool,
}

/// A layer: how many nodes it shows, and how many did not fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer {
    pub layer: i32,
    pub len: usize,
    pub hidden: usize,
}

/// The shown nodes and edges. See the module docs for how they are found and ordered.
#[derive(Debug, Clone)]
pub struct Layout<N> {
    pub nodes: Vec<Placed<N>>,
    pub edges: Vec<LaidEdge>,
    /// Left to right.
    pub layers: Vec<Layer>,
    index: HashMap<N, usize>,
}

impl<N: Copy + Eq + std::hash::Hash> Layout<N> {
    /// The index of `node` in [`Self::nodes`], if shown.
    #[inline]
    pub fn index_of(&self, node: N) -> Option<usize> {
        self.index.get(&node).copied()
    }

    /// The vertical center of each node: the nodes of a layer stacked by row, `heights[i]` tall
    /// with `gap` between, the layer centered on `0`.
    ///
    /// e.g. a layer of heights `[20, 40]`, gap `10` -> centers `-25`, `15`
    #[must_use]
    pub fn stack(&self, heights: &[f32], gap: f32) -> Vec<f32> {
        let mut centers = vec![0.0; self.nodes.len()];
        let mut by_layer: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
        for (i, placed) in self.nodes.iter().enumerate() {
            by_layer.entry(placed.layer).or_default().push(i);
        }
        for mut layer in by_layer.into_values() {
            layer.sort_by_key(|&i| self.nodes[i].row);
            #[expect(clippy::cast_precision_loss, reason = "a few rows")]
            let total =
                gap.mul_add(layer.len().saturating_sub(1) as f32, layer.iter().map(|&i| heights[i]).sum::<f32>());
            let mut top = -total / 2.0;
            for i in layer {
                centers[i] = top + heights[i] / 2.0;
                top += heights[i] + gap;
            }
        }
        centers
    }
}

/// How many times the rows are sorted both ways: more barely changes the order.
const SWEEPS: usize = 4;

/// The neighborhood of `focus`. `keep(node, key)`: is edge `key` of `node` followed and drawn?
/// See the module docs.
pub fn layout<S: GraphSource>(
    source: &S,
    focus: S::Node,
    options: LayoutOptions,
    keep: Keep<'_, S::Node>,
) -> Layout<S::Node> {
    let found = if options.all { find_all(source, keep) } else { find_layers(source, (focus, keep), options) };

    let mut nodes: Vec<Placed<S::Node>> =
        found.order.iter().map(|&(node, layer)| Placed { node, layer, row: 0 }).collect();
    let index: HashMap<S::Node, usize> = nodes.iter().enumerate().map(|(i, placed)| (placed.node, i)).collect();
    let edges = shown_edges(source, (&nodes, &index), keep, &found.parent_of);
    deepen(&mut nodes, &edges);

    let mut rows = rows_by_layer(&nodes);
    order_rows(&mut rows, &nodes, &edges);
    for layer_rows in rows.values() {
        for (row, &i) in layer_rows.iter().enumerate() {
            nodes[i].row = row;
        }
    }

    let layers = rows
        .iter()
        .map(|(&layer, layer_rows)| {
            let hidden =
                found.hidden.get(&layer).map_or(0, |hidden| hidden.iter().filter(|n| !index.contains_key(n)).count());
            Layer { layer, len: layer_rows.len(), hidden }
        })
        .collect();
    Layout { nodes, edges, layers, index }
}

/// What [`find_layers`] found.
struct Found<N> {
    /// The shown nodes in the order they were met, with their layer.
    order: Vec<(N, i32)>,
    layer_of: HashMap<N, i32>,
    /// The node each was found from.
    parent_of: HashMap<N, N>,
    /// The nodes of each layer that did not fit (some may be shown in another).
    hidden: BTreeMap<i32, HashSet<N>>,
}

fn find_layers<S: GraphSource>(
    source: &S,
    (focus, keep): (S::Node, Keep<'_, S::Node>),
    options: LayoutOptions,
) -> Found<S::Node> {
    let mut found = Found {
        order: vec![(focus, 0)],
        layer_of: HashMap::from_iter([(focus, 0)]),
        parent_of: HashMap::default(),
        hidden: BTreeMap::new(),
    };
    let max = options.max_per_layer;
    walk(&mut found, (focus, options.depth_down, 1, max), |node| {
        source.edges(node).filter(move |edge| keep(node, edge.key)).map(|edge| edge.to)
    });
    walk(&mut found, (focus, options.depth_up, -1, max), |node| source.parents(node));
    found
}

/// Every node of `source`: the roots (no parent) in layer 0, and what they point to right of them;
/// then each node not reached (only in a cycle), in layer 0 too.
fn find_all<S: GraphSource>(source: &S, keep: Keep<'_, S::Node>) -> Found<S::Node> {
    let mut found = Found {
        order: Vec::new(),
        layer_of: HashMap::default(),
        parent_of: HashMap::default(),
        hidden: BTreeMap::new(),
    };
    let nodes: Vec<S::Node> = source.nodes().collect();
    let roots = nodes.iter().filter(|&&node| source.parents(node).next().is_none());
    let rest = nodes.iter().filter(|&&node| source.parents(node).next().is_some());
    for &start in roots.chain(rest) {
        if found.layer_of.contains_key(&start) {
            continue;
        }
        found.layer_of.insert(start, 0);
        found.order.push((start, 0));
        walk(&mut found, (start, usize::MAX, 1, usize::MAX), |node| {
            source.edges(node).filter(move |edge| keep(node, edge.key)).map(|edge| edge.to)
        });
    }
    found
}

/// A breadth-first walk from `from` for `depth` layers, each `step` (`1`: right, `-1`: left) from the
/// last, along `next`.
fn walk<N, I>(found: &mut Found<N>, (from, depth, step, max): (N, usize, i32, usize), next: impl Fn(N) -> I)
where
    N: Copy + Eq + std::hash::Hash,
    I: Iterator<Item = N>,
{
    let mut counts: HashMap<i32, usize> = HashMap::default();
    let mut queue = VecDeque::from([(from, 0_usize)]);
    while let Some((node, d)) = queue.pop_front() {
        if d >= depth {
            continue;
        }
        let layer = found.layer_of[&node] + step;
        for to in next(node) {
            if found.layer_of.contains_key(&to) {
                continue;
            }
            let count = counts.entry(layer).or_default();
            if *count >= max {
                found.hidden.entry(layer).or_default().insert(to);
                continue;
            }
            *count += 1;
            found.layer_of.insert(to, layer);
            found.parent_of.insert(to, node);
            found.order.push((to, layer));
            queue.push_back((to, d + 1));
        }
    }
}

/// Is edge `key` of a node followed and drawn? See [`layout`].
pub type Keep<'a, N> = &'a dyn Fn(N, usize) -> bool;

/// The shown nodes, and the index of each.
type Shown<'a, N> = (&'a [Placed<N>], &'a HashMap<N, usize>);

/// Every kept edge between two shown nodes. An edge into the focus is always kept: its parents
/// would stand there unjoined. The first edge between a node and the one it was found from is of
/// the tree.
fn shown_edges<S: GraphSource>(
    source: &S,
    (nodes, index): Shown<'_, S::Node>,
    keep: Keep<'_, S::Node>,
    parent_of: &HashMap<S::Node, S::Node>,
) -> Vec<LaidEdge> {
    let mut edges = Vec::new();
    let mut tree_pairs: HashSet<(usize, usize)> = HashSet::default();
    for (from, placed) in nodes.iter().enumerate() {
        for edge in source.edges(placed.node) {
            let Some(&to) = index.get(&edge.to) else { continue };
            if to != 0 && !keep(placed.node, edge.key) {
                continue;
            }
            let found_along =
                parent_of.get(&edge.to) == Some(&placed.node) || parent_of.get(&placed.node) == Some(&edge.to);
            let tree = found_along && tree_pairs.insert((from, to));
            edges.push(LaidEdge { from, to, label: edge.label.into(), key: edge.key, tree });
        }
    }
    edges
}

/// Moves each node right of the deepest node on the focus's side pointing to it. See the module
/// docs.
fn deepen<N>(nodes: &mut [Placed<N>], edges: &[LaidEdge]) {
    for _ in 0..nodes.len() {
        let mut moved = false;
        for edge in edges.iter().filter(|edge| edge.from != edge.to) {
            let (from, to) = (nodes[edge.from].layer, nodes[edge.to].layer);
            if from >= 0 && to > 0 && to <= from {
                nodes[edge.to].layer = from + 1;
                moved = true;
            }
        }
        if !moved {
            return;
        }
    }
}

/// The node indices of each layer, in the order they were met.
fn rows_by_layer<N>(nodes: &[Placed<N>]) -> BTreeMap<i32, Vec<usize>> {
    let mut rows: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, placed) in nodes.iter().enumerate() {
        rows.entry(placed.layer).or_default().push(i);
    }
    rows
}

/// Sorts each layer by the mean place of its neighbors: see the module docs.
fn order_rows<N>(rows: &mut BTreeMap<i32, Vec<usize>>, nodes: &[Placed<N>], edges: &[LaidEdge]) {
    let mut neighbors = vec![Vec::new(); nodes.len()];
    for edge in edges.iter().filter(|edge| edge.from != edge.to) {
        neighbors[edge.from].push(edge.to);
        neighbors[edge.to].push(edge.from);
    }

    let layer_of: Vec<i32> = nodes.iter().map(|placed| placed.layer).collect();
    let layers: Vec<i32> = rows.keys().copied().collect();
    for _ in 0..SWEEPS {
        for &layer in layers.iter().skip(1) {
            sort_by_neighbors(rows, (layer, true), (&layer_of, &neighbors));
        }
        for &layer in layers.iter().rev().skip(1) {
            sort_by_neighbors(rows, (layer, false), (&layer_of, &neighbors));
        }
    }
}

/// Sorts layer `layer` by the mean place of each node's neighbors in the layers `before` it (or
/// after it). A place is from `0` (top) to `1` (bottom) of its layer, so that layers of different
/// lengths compare. A node with no such neighbor keeps its place.
fn sort_by_neighbors(
    rows: &mut BTreeMap<i32, Vec<usize>>,
    (layer, before): (i32, bool),
    (layer_of, neighbors): (&[i32], &[Vec<usize>]),
) {
    let place = places(rows, layer_of.len());
    let Some(layer_rows) = rows.get_mut(&layer) else { return };
    let is_side = |other: i32| if before { other < layer } else { other > layer };

    #[expect(clippy::cast_precision_loss, reason = "a few rows")]
    let mut keyed: Vec<(f32, usize)> = layer_rows
        .iter()
        .map(|&i| {
            let (sum, count) = neighbors[i]
                .iter()
                .filter(|&&n| is_side(layer_of[n]))
                .fold((0.0, 0_usize), |(sum, count), &n| (sum + place[n], count + 1));
            let key = if count == 0 { place[i] } else { sum / count as f32 };
            (key, i)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (slot, (_, i)) in layer_rows.iter_mut().zip(keyed) {
        *slot = i;
    }
}

/// Each node's place in its layer, from `0` to `1`: the middle of its row.
fn places(rows: &BTreeMap<i32, Vec<usize>>, len: usize) -> Vec<f32> {
    let mut place = vec![0.5; len];
    for layer_rows in rows.values() {
        #[expect(clippy::cast_precision_loss, reason = "a few rows")]
        let count = layer_rows.len() as f32;
        for (row, &i) in layer_rows.iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "a few rows")]
            let at = (row as f32 + 0.5) / count;
            place[i] = at;
        }
    }
    place
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Card, Edge};

    /// Nodes `0..n`, edges `(from, to)`.
    struct Mock(Vec<(u32, u32)>);

    impl GraphSource for Mock {
        type Node = u32;

        fn version(&self) -> u64 {
            0
        }

        fn nodes(&self) -> impl Iterator<Item = u32> {
            let mut nodes: Vec<u32> = self.0.iter().flat_map(|&edge| <[u32; 2]>::from(edge)).collect();
            nodes.sort_unstable();
            nodes.dedup();
            nodes.into_iter()
        }

        fn edges(&self, node: u32) -> impl Iterator<Item = Edge<'_, u32>> {
            self.0.iter().filter(move |e| e.0 == node).map(|&(_, to)| Edge { to, label: "", key: to as usize })
        }

        fn parents(&self, node: u32) -> impl Iterator<Item = u32> {
            self.0.iter().filter(move |e| e.1 == node).map(|e| e.0)
        }

        fn card(&self, _: u32) -> Card<'_> {
            Card { title: "", subtitle: "", header: None, accent: None, dimmed: false, editable: true, fan_in: 0 }
        }
    }

    fn layer_of(layout: &Layout<u32>, node: u32) -> Option<i32> {
        layout.index_of(node).map(|i| layout.nodes[i].layer)
    }

    #[test]
    fn parents_go_left_and_children_right() {
        // 9 -> 0 -> 1 -> 2
        let source = Mock(vec![(9, 0), (0, 1), (1, 2)]);
        let layout = layout(&source, 0, LayoutOptions::DEFAULT, &|_, _| true);
        assert_eq!([9, 0, 1, 2].map(|n| layer_of(&layout, n)), [Some(-1), Some(0), Some(1), Some(2)]);
        assert_eq!(layout.edges.len(), 3);
    }

    #[test]
    fn a_cycle_ends_where_it_was_met() {
        // 0 -> 1 -> 2 -> 0
        let source = Mock(vec![(0, 1), (1, 2), (2, 0)]);
        let layout = layout(&source, 0, LayoutOptions::DEFAULT, &|_, _| true);
        assert_eq!(layout.nodes.len(), 3, "{:?}", layout.nodes);
        assert_eq!(layer_of(&layout, 2), Some(2), "met going down first");
        assert_eq!(layout.edges.len(), 3, "the edge back to the focus is drawn too");
    }

    #[test]
    fn a_shared_node_goes_right_of_its_deepest_parent() {
        // 0 -> 1 -> 2, and 0 -> 2: 2 is met in layer 1 first.
        let source = Mock(vec![(0, 1), (0, 2), (1, 2)]);
        let layout = layout(&source, 0, LayoutOptions::DEFAULT, &|_, _| true);
        assert_eq!([1, 2].map(|n| layer_of(&layout, n)), [Some(1), Some(2)]);

        let tree: Vec<(u32, u32, bool)> = layout
            .edges
            .iter()
            .map(|edge| (layout.nodes[edge.from].node, layout.nodes[edge.to].node, edge.tree))
            .collect();
        assert!(tree.contains(&(0, 2, true)), "2 was found from 0: {tree:?}");
        assert!(tree.contains(&(1, 2, false)), "the other way to 2 crosses: {tree:?}");
    }

    #[test]
    fn edges_not_kept_are_not_followed() {
        // 0 -> 1 (kept), 0 -> 2 (not): 2 is not shown, and no edge to it.
        let source = Mock(vec![(0, 1), (0, 2)]);
        let layout = layout(&source, 0, LayoutOptions::DEFAULT, &|_, key| key != 2);
        assert_eq!(layer_of(&layout, 2), None);
        assert_eq!(layout.edges.len(), 1);
    }

    #[test]
    fn depth_and_a_full_layer_hide_the_rest() {
        let mut edges: Vec<(u32, u32)> = (1..=5).map(|child| (0, child)).collect();
        edges.push((1, 10));
        let source = Mock(edges);
        let options = LayoutOptions { depth_up: 0, depth_down: 1, max_per_layer: 3, all: false };
        let layout = layout(&source, 0, options, &|_, _| true);
        assert_eq!(layer_of(&layout, 10), None, "beyond the depth");
        let first = layout.layers.iter().find(|layer| layer.layer == 1).copied();
        assert_eq!(first, Some(Layer { layer: 1, len: 3, hidden: 2 }));
    }

    #[test]
    fn rows_follow_their_parents() {
        // 0 -> 1, 2; 1 -> 4; 2 -> 3; and 5 -> 3, 4 met last: it goes between them, under nothing.
        let source = Mock(vec![(0, 1), (0, 2), (2, 3), (1, 4), (5, 3), (5, 4)]);
        let layout = layout(&source, 0, LayoutOptions::DEFAULT, &|_, _| true);
        let row = |node| layout.index_of(node).map(|i| layout.nodes[i].row);
        assert!(row(1) < row(2));
        assert!(row(4) < row(3), "under their parents: {:?}", layout.nodes);
    }

    /// The whole graph: every root in layer 0 and what it points to right of it, wherever the
    /// focus is; a cycle no root reaches is shown too.
    #[test]
    fn all_the_nodes_from_the_roots() {
        // 0 -> 1 -> 2, 5 -> 2, and 7 <-> 8 apart.
        let source = Mock(vec![(0, 1), (1, 2), (5, 2), (7, 8), (8, 7)]);
        let options = LayoutOptions { all: true, ..LayoutOptions::DEFAULT };
        let layout = layout(&source, 2, options, &|_, _| true);
        let layers = [0, 1, 2, 5, 7, 8].map(|n| layer_of(&layout, n));
        assert_eq!(layers, [Some(0), Some(1), Some(2), Some(0), Some(0), Some(1)]);
        assert_eq!(layout.edges.len(), 5);
    }

    #[test]
    fn layers_are_stacked_around_the_middle() {
        let source = Mock(vec![(0, 1), (0, 2)]);
        let layout = layout(&source, 0, LayoutOptions::DEFAULT, &|_, _| true);
        let heights = [30.0, 20.0, 40.0];
        let centers = layout.stack(&heights, 10.0);
        assert!(centers[0].abs() < f32::EPSILON, "the focus is alone in its layer");
        let mut children = [centers[1], centers[2]];
        children.sort_by(f32::total_cmp);
        assert_eq!(children, [-25.0, 15.0], "20 + 10 + 40 = 70: from -35");
    }
}
