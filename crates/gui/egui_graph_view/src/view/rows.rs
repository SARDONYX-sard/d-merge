//! The rows of a node as the view keeps them between layouts, and the pages of its long lists.
//!
//! ```text
//! source rows                      shown (a page of 8)
//! generators[0]                    │ generators   9-16 / 1000  ‹ › │  a pager
//! ...                        ->    │ generators[8]             ●   │
//! generators[999]                  │ ...                           │
//! generators +                     │ generators[15]            ●   │
//!                                  │ generators                +●  │
//! ```
//!
//! An edge from a row on no shown page is neither followed by the layout nor drawn.

use egui::Color32;
use fast_hash::HashMap;

use crate::Row;

/// What the view keeps of a row: the rest is read from the source when it is drawn.
#[derive(Debug, Clone, Copy)]
pub(super) struct RowMeta {
    pub(super) key: usize,
    pub(super) color: Option<Color32>,
    /// The list it is an element of: an index in [`NodeRows::lists`].
    pub(super) list: Option<usize>,
}

/// Rows of one label with an index, one after another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct List {
    /// Its first row.
    pub(super) start: usize,
    pub(super) len: usize,
}

/// What is drawn on a line of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Line {
    Row(usize),
    /// The pager of list `.0`.
    Pager(usize),
}

/// The rows of a node.
#[derive(Debug, Default)]
pub(super) struct NodeRows {
    pub(super) rows: Vec<RowMeta>,
    pub(super) lists: Vec<List>,
    /// The row of each key.
    index: HashMap<usize, usize>,
}

impl NodeRows {
    pub(super) fn read<'a>(rows: impl Iterator<Item = Row<'a>>) -> Self {
        let mut node = Self::default();
        let mut last_label: Option<&str> = None;
        for (at, row) in rows.enumerate() {
            let list = row.index.map(|_| {
                let continues =
                    last_label == Some(row.label) && node.rows.last().is_some_and(|last| last.list.is_some());
                if !continues {
                    node.lists.push(List { start: at, len: 0 });
                }
                let list = node.lists.len() - 1;
                node.lists[list].len += 1;
                list
            });
            last_label = Some(row.label);
            node.index.insert(row.key, at);
            node.rows.push(RowMeta { key: row.key, color: row.color, list });
        }
        node
    }

    /// The row of `key`, if any.
    #[inline]
    pub(super) fn row_of(&self, key: usize) -> Option<usize> {
        self.index.get(&key).copied()
    }

    /// The rows of list `list` on page `page` (the last page, if beyond it).
    pub(super) fn page(&self, list: usize, page: usize, per_page: usize) -> std::ops::Range<usize> {
        let List { start, len } = self.lists[list];
        let last = len.saturating_sub(1) / per_page.max(1);
        let from = page.min(last) * per_page;
        start + from..start + (from + per_page).min(len)
    }

    /// Is list `list` long enough to be paged?
    #[inline]
    pub(super) fn is_paged(&self, list: usize, per_page: usize) -> bool {
        self.lists[list].len > per_page
    }

    /// Is the row of `key` shown, with `page_of` each list's page? Keys of no row are.
    pub(super) fn is_shown(&self, key: usize, page_of: impl Fn(usize) -> usize, per_page: usize) -> bool {
        let Some(row) = self.row_of(key) else { return true };
        match self.rows[row].list {
            Some(list) if self.is_paged(list, per_page) => self.page(list, page_of(list), per_page).contains(&row),
            _ => true,
        }
    }

    /// The lines drawn: each row, but a long list as its pager and its page's rows.
    pub(super) fn lines(&self, page_of: impl Fn(usize) -> usize, per_page: usize) -> Vec<Line> {
        let mut lines = Vec::with_capacity(self.rows.len().min(64));
        let mut row = 0;
        while row < self.rows.len() {
            match self.rows[row].list {
                Some(list) if self.is_paged(list, per_page) => {
                    lines.push(Line::Pager(list));
                    lines.extend(self.page(list, page_of(list), per_page).map(Line::Row));
                    row = self.lists[list].start + self.lists[list].len;
                }
                _ => {
                    lines.push(Line::Row(row));
                    row += 1;
                }
            }
        }
        lines
    }

    /// `(list, page)` that shows the row of `key`, if it is on a paged list.
    pub(super) fn page_of_key(&self, key: usize, per_page: usize) -> Option<(usize, usize)> {
        let row = self.row_of(key)?;
        let list = self.rows[row].list.filter(|&list| self.is_paged(list, per_page))?;
        Some((list, (row - self.lists[list].start) / per_page.max(1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Slot;

    /// `name`, then `items[0..len]` (keys `10..`), then `items +`.
    fn node(len: usize) -> NodeRows {
        let slot = Slot::Pin { connected: true };
        let name = Row { label: "name", index: None, slot: Slot::Value("x"), color: None, key: 0 };
        let items = (0..len).map(|i| Row { label: "items", index: Some(i), slot, color: None, key: 10 + i });
        let add = Row { label: "items", index: None, slot: Slot::AddPin, color: None, key: 1 };
        NodeRows::read(std::iter::once(name).chain(items).chain(std::iter::once(add)))
    }

    #[test]
    fn a_short_list_is_all_shown() {
        let node = node(3);
        assert_eq!(node.lists, [List { start: 1, len: 3 }]);
        assert_eq!(node.lines(|_| 0, 8).len(), 5);
        assert!(node.is_shown(12, |_| 0, 8));
    }

    #[test]
    fn a_long_list_shows_a_page() {
        let node = node(1000);
        let lines = node.lines(|_| 1, 8);
        assert_eq!(lines[..3], [Line::Row(0), Line::Pager(0), Line::Row(9)], "page 1: rows 8..16 of the list");
        assert_eq!(lines.len(), 1 + 1 + 8 + 1, "name, pager, a page, `+`");
        assert!(node.is_shown(10 + 8, |_| 1, 8));
        assert!(!node.is_shown(10, |_| 1, 8), "page 0's");
        assert!(node.is_shown(1, |_| 1, 8), "`+` is not of the list");
        assert_eq!(node.page_of_key(10 + 999, 8), Some((0, 124)));
        assert_eq!(node.page(0, 999, 8), 993..1001, "beyond the last: the last");
    }
}
