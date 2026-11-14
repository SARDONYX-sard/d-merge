//! A tree's state on screen, as VS Code's explorer keeps it: which nodes are open, the rows they make,
//! the cursor, the scrolling. What the nodes are is the host's ([`TreeSource`]).
//!
//! ```text
//! ▼ #0090 root              rows: what is open, depth first; a node that is also under another
//!   ▼ #0091 graph                 parent is shown there dimmed, as a leaf (`TreeRow::dimmed`)
//!     ▶ #0094 data       <- cursor: follows the selection; `j` `k` ... move it (`apply_keys`)
//!   ...
//! ```
//! Each frame:
//! ```text
//! view.prepare(&source, selected);                       // rows up to date, cursor on `selected`
//! let keys = view.apply_keys(&source, &keys);            // the keyboard: keys.selected, keys.leave
//! for row in view.rows() { /* draw it; guides::paint(ui, row.lines(), ..) */ }
//! view.scroll.note(&output);                             // where the list is now
//! ```
//! - Open / closed is kept by the node's key ([`TreeSource::key`]), so it survives edits, and can be
//!   saved ([`TreeView::take_open_changes`]) and restored ([`TreeView::restored`]).
//! - The rows are built again only when the source's [`TreeSource::version`] or what is open changed.
//! - A selection made elsewhere moves only the cursor; [`TreeView::reveal_again`] (e.g. the editor got
//!   the keyboard) also opens the way to it and scrolls there, once.

use std::{collections::BTreeMap, ops::RangeInclusive};

use crate::{Nav, NavRow, TreeKey, guides::GuideRow, list::ListScroll};

/// The nodes of a tree, as the host has them. e.g. objects pointing to objects.
pub trait TreeSource {
    /// A node, e.g. an index into the host's list.
    type Node: Copy + Eq;

    /// The top node; `None`: the tree is empty.
    fn root(&self) -> Option<Self::Node>;

    /// The children of `node` shown, in order.
    fn children(&self, node: Self::Node) -> impl Iterator<Item = Self::Node>;

    /// The parent `node` opens under; under any other it is a dimmed leaf. `None`: the root.
    fn parent(&self, node: Self::Node) -> Option<Self::Node>;

    /// What names `node` across edits, e.g. its id `#0106`: what is open is kept by it.
    fn key(&self, node: Self::Node) -> &str;

    /// The node of `key`, if there is one.
    fn node_of(&self, key: &str) -> Option<Self::Node>;

    /// Is `node` open before it was opened or closed by hand? e.g. only the root.
    fn default_open(&self, node: Self::Node) -> bool;

    /// Changes whenever the rows would: a node added, removed or moved, a filter changed. e.g. a
    /// counter of the host's edits that touch the tree.
    fn version(&self) -> u64;
}

/// A line of the tree on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow<N> {
    pub node: N,
    /// `0`: the root.
    pub depth: u16,
    /// It has children shown: it opens.
    pub expandable: bool,
    pub open: bool,
    /// Opens under another parent: here a leaf, drawn dimmed.
    pub dimmed: bool,
    /// The last child of its parent: its line ends here (`└`, not `├`).
    pub last_child: bool,
    /// The row of its parent. `None`: the root.
    pub parent_row: Option<u32>,
    /// See [`GuideRow::guides`].
    pub guides: u64,
}

impl<N> TreeRow<N> {
    /// What the keys need of it.
    #[inline]
    #[must_use]
    pub fn nav(&self) -> NavRow {
        NavRow { depth: usize::from(self.depth), open: self.expandable.then_some(self.open) }
    }

    /// What its lines need of it: see [`crate::guides::paint`].
    #[inline]
    #[must_use]
    pub const fn lines(&self) -> GuideRow {
        GuideRow { depth: self.depth, last_child: self.last_child, guides: self.guides }
    }
}

/// What [`TreeView::apply_keys`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeysOutput<N> {
    /// The cursor moved to this node: the host selects it (as VS Code's explorer opens the file).
    pub selected: Option<N>,
    /// `Enter` / `Esc`: the keyboard goes back to where it came from.
    pub leave: bool,
}

/// See the module docs.
#[derive(Debug)]
pub struct TreeView<N> {
    /// What was opened / closed by hand, by key. Others: [`TreeSource::default_open`].
    open: BTreeMap<Box<str>, bool>,
    /// `open` changed since [`Self::take_open_changes`].
    open_changed: bool,
    rows: Vec<TreeRow<N>>,
    /// The source's version the rows were built from; `None`: build them again.
    built_for: Option<u64>,
    /// The row the keyboard is on.
    pub cursor: usize,
    /// See [`Self::reveal_again`].
    reveal_asked: bool,
    /// The selection the cursor was put on.
    cursor_for: Option<Box<str>>,
    /// Bring this row into view on the next draw ([`Self::offset_for`]).
    pub scroll_to: Option<usize>,
    /// Where the list is scrolled up and down: note it after each draw.
    pub scroll: ListScroll,
    /// Where it is scrolled sideways, and how wide it is seen: see [`Self::offset_x_for`].
    pub scroll_x: f32,
    pub view_width: f32,
}

impl<N> Default for TreeView<N> {
    fn default() -> Self {
        Self {
            open: BTreeMap::new(),
            open_changed: false,
            rows: Vec::new(),
            built_for: None,
            cursor: 0,
            reveal_asked: false,
            cursor_for: None,
            scroll_to: None,
            scroll: ListScroll::default(),
            scroll_x: 0.0,
            view_width: 0.0,
        }
    }
}

impl<N: Copy + Eq> TreeView<N> {
    /// A tree with what was open / closed by hand, e.g. from the settings.
    pub fn restored<'a>(open: impl IntoIterator<Item = (&'a str, bool)>) -> Self {
        Self { open: open.into_iter().map(|(key, open)| (key.into(), open)).collect(), ..Self::default() }
    }

    /// What is open / closed by hand, to save, if it changed since the last call.
    pub fn take_open_changes(&mut self) -> Option<impl Iterator<Item = (&str, bool)>> {
        std::mem::take(&mut self.open_changed).then(|| self.open.iter().map(|(key, open)| (&**key, *open)))
    }

    #[inline]
    #[must_use]
    pub fn rows(&self) -> &[TreeRow<N>] {
        &self.rows
    }

    /// Is `node` open, by hand or by default?
    pub fn is_open<S: TreeSource<Node = N>>(&self, source: &S, node: N) -> bool {
        self.open.get(source.key(node)).copied().unwrap_or_else(|| source.default_open(node))
    }

    /// Opens or closes `node`. The rows are built again on the next [`Self::prepare`].
    pub fn set_open<S: TreeSource<Node = N>>(&mut self, source: &S, node: N, open: bool) {
        self.open.insert(source.key(node).into(), open);
        self.open_changed = true;
        self.built_for = None;
    }

    /// Opens or closes `node` and everything that opens under it.
    pub fn set_open_below<S: TreeSource<Node = N>>(&mut self, source: &S, node: N, open: bool) {
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            self.set_open(source, current, open);
            stack.extend(source.children(current).filter(|&child| source.parent(child) == Some(current)));
        }
    }

    /// The editor got the keyboard: the next [`Self::prepare`] opens the way to the selection and
    /// scrolls to it, once. Otherwise a tree being used never moves by itself.
    #[inline]
    pub const fn reveal_again(&mut self) {
        self.reveal_asked = true;
    }

    /// Builds the rows again if they are stale, and puts the cursor on `selected` (a key).
    ///
    /// ```text
    /// selected changed (a click, a tab, `gd`)  -> the cursor follows; nothing moves
    /// after `reveal_again`                    -> its parents open, and it is scrolled to
    /// ```
    pub fn prepare<S: TreeSource<Node = N>>(&mut self, source: &S, selected: Option<&str>) {
        let revealed = std::mem::take(&mut self.reveal_asked) && self.reveal(source, selected);

        let stale = self.built_for != Some(source.version());
        if stale {
            self.rebuild(source);
        }

        let moved = self.cursor_for.as_deref() != selected;
        if revealed || stale || moved {
            self.cursor_for = selected.map(Into::into);
            let node = selected.and_then(|key| source.node_of(key));
            let row = node.and_then(|node| self.rows.iter().position(|row| row.node == node && !row.dimmed));
            if let Some(row) = row {
                self.cursor = row;
                if revealed {
                    self.scroll_to = Some(row);
                }
            }
        }
    }

    /// Runs this frame's keys ([`crate::navigate`]): moves the cursor, opens and closes.
    pub fn apply_keys<S: TreeSource<Node = N>>(&mut self, source: &S, keys: &[TreeKey]) -> KeysOutput<N> {
        let before = self.cursor;
        let mut leave = false;

        for &key in keys {
            let nav: Vec<NavRow> = self.rows.iter().map(TreeRow::nav).collect();
            match crate::navigate(key, &nav, self.cursor) {
                Nav::Cursor(row) => self.cursor = row,
                Nav::SetOpen(row, open) => {
                    self.set_open(source, self.rows[row].node, open);
                    self.rebuild(source);
                }
                Nav::Leave => leave = true,
                Nav::None => {}
            }
        }

        let mut selected = None;
        if self.cursor != before
            && let Some(row) = self.rows.get(self.cursor)
        {
            self.scroll_to = Some(self.cursor);
            self.cursor_for = Some(source.key(row.node).into());
            selected = Some(row.node);
        }
        KeysOutput { selected, leave }
    }

    /// The rows of `row`'s ancestors, root first: the way from the root to it.
    ///
    /// e.g. for `#c` in `#root > #b > #c`: the rows of `#root` and `#b`.
    #[must_use]
    pub fn ancestors(&self, row: usize) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut current = self.rows.get(row).and_then(|row| row.parent_row);
        while let Some(parent) = current {
            chain.push(parent as usize);
            current = self.rows[parent as usize].parent_row;
        }
        chain.reverse();
        chain
    }

    /// The rows to pin over the top of the view, as VS Code's sticky scroll: the ancestors of what is
    /// shown under them, at most the `max` nearest. `step`: a row's height with its spacing.
    #[must_use]
    pub fn sticky_rows(&self, step: f32, max: usize) -> Vec<usize> {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a positive row number")]
        let top = (self.scroll.offset / step).max(0.0) as usize;

        // The pinned rows cover as many rows as they are: go by the first row still seen.
        let covered = self.ancestors(top).len().min(max);
        let chain = self.ancestors((top + covered).min(self.rows.len().saturating_sub(1)));
        chain[chain.len().saturating_sub(max)..].to_vec()
    }

    /// The way to the cursor's row on the lines between rows: per level, the rows whose line at
    /// that level is on it, for [`crate::guides::paint`]. e.g. `[Some(0..=3), Some(1..=2)]`.
    #[must_use]
    pub fn active_guides(&self) -> Vec<Option<RangeInclusive<usize>>> {
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

    /// The sideways offset that shows the start of a row indented by `indent` pixels; `None`: it is
    /// seen. Deep rows would be out of view to the right.
    #[must_use]
    pub fn offset_x_for(&self, indent: f32) -> Option<f32> {
        const MARGIN: f32 = 24.0;
        let visible = self.scroll_x..=self.scroll_x + self.view_width / 2.0;
        (!visible.contains(&indent)).then_some((indent - MARGIN).max(0.0))
    }

    /// The offset that brings `row` into view below its ancestors pinned over the top (at most
    /// `max_sticky`); `None`: it is seen. `sizes`: see [`crate::list::offset_to_show`].
    #[must_use]
    pub fn offset_for(&self, row: usize, sizes: (f32, f32), max_sticky: usize) -> Option<f32> {
        let covered = self.ancestors(row).len().min(max_sticky);
        crate::list::offset_to_show(row, sizes, self.rows.len(), self.scroll, covered)
    }

    /// Opens the parents of `selected`. Returns whether it is in the tree.
    fn reveal<S: TreeSource<Node = N>>(&mut self, source: &S, selected: Option<&str>) -> bool {
        let Some(target) = selected.and_then(|key| source.node_of(key)) else { return false };

        let mut parent = source.parent(target);
        while let Some(node) = parent {
            if !self.is_open(source, node) {
                self.set_open(source, node, true);
            }
            parent = source.parent(node);
        }
        true
    }

    fn rebuild<S: TreeSource<Node = N>>(&mut self, source: &S) {
        let mut rows = std::mem::take(&mut self.rows);
        rows.clear();
        if let Some(root) = source.root() {
            self.push_rows(source, root, (0, None, true, 0), &mut rows);
        }
        self.rows = rows;
        self.built_for = Some(source.version());
    }

    /// The row of `node` and, if open, of what is under it. `(depth, parent_row, last_child, guides)`:
    /// where it hangs.
    fn push_rows<S: TreeSource<Node = N>>(
        &self,
        source: &S,
        node: N,
        (depth, parent_row, last_child, guides): (u16, Option<u32>, bool, u64),
        rows: &mut Vec<TreeRow<N>>,
    ) {
        let expandable = source.children(node).next().is_some();
        let open = expandable && self.is_open(source, node);

        #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion rows")]
        let this_row = Some(rows.len() as u32);
        rows.push(TreeRow { node, depth, expandable, open, dimmed: false, last_child, parent_row, guides });
        if !open {
            return;
        }

        let children: Vec<N> = source.children(node).collect();
        for (at, &child) in children.iter().enumerate() {
            let last_child = at + 1 == children.len();
            // This node's children hang from a line at this depth: more of them keep it going.
            let guides = if last_child { guides } else { guides | crate::guides::bit(depth) };

            if source.parent(child) == Some(node) {
                self.push_rows(source, child, (depth + 1, this_row, last_child, guides), rows);
            } else {
                let row = TreeRow {
                    node: child,
                    depth: depth + 1,
                    expandable: false,
                    open: false,
                    dimmed: true,
                    last_child,
                    parent_row: this_row,
                    guides,
                };
                rows.push(row);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `root > a > c`, `root > b > (c, d)`: `c` opens under `a`, and is dimmed under `b`.
    struct Sample;

    const KEYS: [&str; 5] = ["root", "a", "b", "c", "d"];

    impl TreeSource for Sample {
        type Node = usize;

        fn root(&self) -> Option<usize> {
            Some(0)
        }

        fn children(&self, node: usize) -> impl Iterator<Item = usize> {
            let children: &[usize] = match node {
                0 => &[1, 2],
                1 => &[3],
                2 => &[3, 4],
                _ => &[],
            };
            children.iter().copied()
        }

        fn parent(&self, node: usize) -> Option<usize> {
            [None, Some(0), Some(0), Some(1), Some(2)][node]
        }

        fn key(&self, node: usize) -> &str {
            KEYS[node]
        }

        fn node_of(&self, key: &str) -> Option<usize> {
            KEYS.iter().position(|known| *known == key)
        }

        fn default_open(&self, node: usize) -> bool {
            node == 0
        }

        fn version(&self) -> u64 {
            1
        }
    }

    fn keys(view: &TreeView<usize>) -> Vec<&'static str> {
        view.rows().iter().map(|row| KEYS[row.node]).collect()
    }

    #[test]
    fn only_the_root_starts_open_and_a_shared_node_opens_once() {
        let mut view = TreeView::default();
        view.prepare(&Sample, None);
        assert_eq!(keys(&view), ["root", "a", "b"]);

        view.set_open_below(&Sample, 0, true);
        view.prepare(&Sample, None);
        assert_eq!(keys(&view), ["root", "a", "c", "b", "c", "d"]);
        assert!(view.rows()[4].dimmed, "`c` under `b`");
        assert_eq!(view.ancestors(5), [0, 3]);
        view.cursor = 5;
        assert_eq!(view.active_guides(), [Some(1..=3), Some(4..=5)]);
    }

    #[test]
    fn a_selection_moves_only_the_cursor_until_revealed() {
        let mut view = TreeView::default();
        view.prepare(&Sample, Some("d"));
        assert_eq!((keys(&view).len(), view.scroll_to), (3, None), "nothing opened, nothing scrolled");

        view.reveal_again();
        view.prepare(&Sample, Some("d"));
        assert_eq!(keys(&view), ["root", "a", "b", "c", "d"]);
        assert_eq!((view.cursor, view.scroll_to), (4, Some(4)));
    }

    #[test]
    fn keys_move_open_and_select() {
        let mut view = TreeView::default();
        view.prepare(&Sample, None);
        let output = view.apply_keys(&Sample, &[TreeKey::Next, TreeKey::Next, TreeKey::Open]);
        assert_eq!(output, KeysOutput { selected: Some(2), leave: false }, "on `b`, opened");
        assert_eq!(keys(&view), ["root", "a", "b", "c", "d"]);
        assert!(view.apply_keys(&Sample, &[TreeKey::Leave]).leave);

        let saved: Vec<(String, bool)> =
            view.take_open_changes().unwrap().map(|(key, open)| (key.to_owned(), open)).collect();
        assert_eq!(saved, [("b".to_owned(), true)]);
        assert!(view.take_open_changes().is_none(), "reported once");
        let restored = TreeView::<usize>::restored(saved.iter().map(|(key, open)| (key.as_str(), *open)));
        assert!(restored.is_open(&Sample, 2));
    }

    #[test]
    fn scrolled_sideways_only_when_the_indent_is_out_of_view() {
        let view = TreeView::<usize> { scroll_x: 100.0, view_width: 200.0, ..TreeView::default() };
        assert_eq!(view.offset_x_for(150.0), None);
        assert_eq!(view.offset_x_for(500.0), Some(476.0));
        assert_eq!(view.offset_x_for(10.0), Some(0.0));
    }
}
