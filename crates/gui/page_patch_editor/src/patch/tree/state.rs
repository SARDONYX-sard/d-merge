//! What the tree shows of the [`Graph`]: its rows, what is open, the cursor and the scrolling.

use std::{collections::BTreeMap, rc::Rc};

use fast_hash::HashMap;

use super::{Graph, Node};
use crate::patch::tool::list::{ListScroll, offset_to_show};

/// A line of the tree on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) struct Row {
    pub(in crate::patch) node: Node,
    pub(in crate::patch) depth: u16,
    pub(in crate::patch) expandable: bool,
    pub(in crate::patch) open: bool,
    /// Expanded at another place.
    pub(in crate::patch) dimmed: bool,
    /// The last child of its parent: its connecting line ends here (`└`, not `├`).
    pub(in crate::patch) last_child: bool,
    /// The row of its parent. `None`: the root.
    pub(in crate::patch) parent_row: Option<u32>,
    /// Bit `n`: the line of tree level `n` continues past this row, as that ancestor has more
    /// children below. e.g., `│   └ row` has bit 0.
    pub(in crate::patch) guides: u64,
}

impl Row {
    /// What the keys need of it: how deep, and whether it opens.
    fn nav(&self) -> egui_tree::NavRow {
        egui_tree::NavRow { depth: usize::from(self.depth), open: self.expandable.then_some(self.open) }
    }
}

/// How a row connects to the rows around it. See [`Row`].
#[derive(Debug, Clone, Copy)]
struct RowLines {
    parent_row: Option<u32>,
    last_child: bool,
    guides: u64,
}

pub(in crate::patch) use egui_tree::TreeKey;

/// How many ancestors are pinned over the tree at most. See [`TreeState::sticky_rows`].
pub(in crate::patch) const MAX_STICKY: usize = 5;

/// Tree state kept between frames.
#[derive(Debug, Default)]
pub(in crate::patch) struct TreeState {
    /// Nodes the user opened / closed, by id so it survives edits. Others: [`Self::default_open`].
    open: HashMap<Rc<str>, bool>,
    /// `open` changed since the settings last took it. See [`Self::take_open_changes`].
    open_changed: bool,
    rows: Vec<Row>,
    /// What `rows` was built from: [`Graph::tree_generation`] and "changed only".
    built_for: Option<(u64, bool)>,
    /// The row of the selected object.
    pub(in crate::patch) cursor: usize,
    /// The editor got the keyboard: the next [`Self::prepare`] opens the way to the selection and
    /// scrolls to it. Only then: a tree being used must not move by itself.
    reveal_asked: bool,
    /// The selection the cursor was put on (it follows a selection made elsewhere, without scrolling).
    cursor_for: Option<Rc<str>>,
    /// Bring this row into view on the next draw.
    pub(in crate::patch) scroll_to: Option<usize>,
    pub(in crate::patch) scroll_offset: f32,
    pub(in crate::patch) view_height: f32,
    pub(in crate::patch) scroll_x: f32,
    pub(in crate::patch) view_width: f32,
}

impl TreeState {
    /// A tree with the open / closed nodes saved in the settings.
    pub(in crate::patch) fn restored(open: &BTreeMap<String, bool>) -> Self {
        let open = open.iter().map(|(id, open)| (Rc::from(id.as_str()), *open)).collect();
        Self { open, ..Self::default() }
    }

    /// The open / closed nodes to save, if they changed since the last call.
    pub(in crate::patch) fn take_open_changes(&mut self) -> Option<BTreeMap<String, bool>> {
        std::mem::take(&mut self.open_changed)
            .then(|| self.open.iter().map(|(id, open)| (id.to_string(), *open)).collect())
    }

    pub(in crate::patch) fn rows(&self) -> &[Row] {
        &self.rows
    }

    fn default_open(graph: &Graph, node: Node, changed_only: bool) -> bool {
        graph.root == Some(node) || changed_only
    }

    fn is_open(&self, graph: &Graph, node: Node, changed_only: bool) -> bool {
        self.open.get(graph.id(node)).copied().unwrap_or_else(|| Self::default_open(graph, node, changed_only))
    }

    pub(in crate::patch) fn set_open(&mut self, graph: &Graph, node: Node, open: bool) {
        self.open.insert(Rc::clone(&graph.entry(node).id), open);
        self.open_changed = true;
        self.built_for = None;
    }

    /// Opens / closes `node` and everything expanding under it.
    pub(in crate::patch) fn set_open_below(&mut self, graph: &Graph, node: Node, open: bool) {
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
    pub(in crate::patch) fn prepare(&mut self, graph: &Graph, selected: Option<&str>, changed_only: bool) {
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
    pub(in crate::patch) const fn reveal_again(&mut self) {
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
            let guides = if last_child { guides } else { guides | egui_tree::guides::bit(depth) };
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
    pub(in crate::patch) fn apply_inputs(
        &mut self,
        inputs: &[TreeKey],
        graph: &Graph,
        selected: &mut Option<Rc<str>>,
        changed_only: bool,
    ) -> bool {
        self.prepare(graph, selected.as_deref(), changed_only);
        let previous = self.cursor;
        let mut leave = false;
        for &input in inputs {
            let rows: Vec<egui_tree::NavRow> = self.rows.iter().map(Row::nav).collect();
            match egui_tree::navigate(input, &rows, self.cursor) {
                egui_tree::Nav::Cursor(cursor) => self.cursor = cursor,
                egui_tree::Nav::SetOpen(at, open) => {
                    let node = self.rows[at].node;
                    self.set_open(graph, node, open);
                    self.rebuild_rows(graph, changed_only);
                }
                egui_tree::Nav::Leave => leave = true,
                egui_tree::Nav::None => {}
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
    pub(in crate::patch) fn ancestors(&self, row: usize) -> Vec<usize> {
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
    pub(in crate::patch) fn sticky_rows(&self, step: f32, max: usize) -> Vec<usize> {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a positive row number")]
        let top = (self.scroll_offset / step).max(0.0) as usize;
        // The pinned rows cover as many rows as they are: go by the first row still seen.
        let covered = self.ancestors(top).len().min(max);
        let chain = self.ancestors((top + covered).min(self.rows.len().saturating_sub(1)));
        chain[chain.len().saturating_sub(max)..].to_vec()
    }

    /// The way to the selected row on the connecting lines: per tree level, the rows whose line at
    /// that level is part of it. e.g., `[Some(0..=3), Some(1..=2)]`.
    pub(in crate::patch) fn active_guides(&self) -> Vec<Option<std::ops::RangeInclusive<usize>>> {
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
    pub(in crate::patch) fn offset_x_for(&self, indent: f32) -> Option<f32> {
        const MARGIN: f32 = 24.0;
        let visible = self.scroll_x..=self.scroll_x + self.view_width / 2.0;
        (!visible.contains(&indent)).then_some((indent - MARGIN).max(0.0))
    }

    /// The scroll offset that brings row `row` into view, below its parents pinned over the top
    /// ([`Self::sticky_rows`]) (`None`: it is seen already). See [`offset_to_show`].
    pub(in crate::patch) fn offset_for(&self, row: usize, sizes: (f32, f32)) -> Option<f32> {
        let covered = self.ancestors(row).len().min(MAX_STICKY);
        let scroll = ListScroll { offset: self.scroll_offset, view: self.view_height };
        offset_to_show(row, sizes, self.rows.len(), scroll, covered)
    }
}
