//! Keyboard-driven trees and lists for egui, as VS Code's explorer.
//!
//! The host keeps its rows (what they are is its own) and gives this crate only what the keys and
//! the layout need:
//!
//! ```text
//! rows:   [NavRow { depth: 0, open: Some(true) },    ▼ Root
//!          NavRow { depth: 1, open: None },             Leaf      <- cursor
//!          NavRow { depth: 1, open: Some(false) }]      ▶ Group
//!
//! navigate(TreeKey::Close, &rows, 1) -> Nav::Cursor(0)   `h` on a leaf: up to its parent
//! ```
//!
//! - [`TreeKey`] / [`navigate`]: what a key does (`j` `k` `g` `G` `h` `l` `o` `Enter`). The host maps
//!   its key bindings to the keys and carries out the [`Nav`] (opens a node, selects a row).
//! - [`list`]: rows of one height in a list that scrolls only up and down (`ScrollArea::show_rows`),
//!   scrolled just enough to show a row, never widening its panel.
//! - [`guides`]: the lines joining the rows, as VS Code's indent guides.

pub mod guides;
pub mod list;

/// A key of a tree or a list, whatever key binding asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeKey {
    /// `j`, Down
    Next,
    /// `k`, Up
    Previous,
    /// `g`
    First,
    /// `G`
    Last,
    /// `h`, Left: closes an open node; from a closed one or a leaf, up to its parent.
    Close,
    /// `l`, Right: opens a closed node; on an open one, into it.
    Open,
    /// `o`: opens or closes a node. The cursor stays.
    Toggle,
    /// `Enter`, `Esc`: the keyboard goes back to where it came from.
    Leave,
}

impl TreeKey {
    /// The row a movement leads to in a list of `len` rows; `None`: not a movement.
    ///
    /// e.g. `Next` on the last row stays there; `First` -> `0`.
    #[inline]
    #[must_use]
    pub const fn step(self, current: usize, len: usize) -> Option<usize> {
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

/// What [`navigate`] needs of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavRow {
    /// How deep: `0` at the top. A row's parent is the nearest row above it that is less deep.
    pub depth: usize,
    /// `Some(open)`: a node that opens and closes; `None`: a leaf.
    pub open: Option<bool>,
}

/// What a key does to a tree: the host carries it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// The keyboard goes to this row.
    Cursor(usize),
    /// The node of this row opens (`true`) or closes. The rows change: lay them out again.
    SetOpen(usize, bool),
    /// Back to where the keyboard came from.
    Leave,
    None,
}

/// `key` on `rows` with the keyboard on row `cursor`. See [`TreeKey`] for what each key does.
#[must_use]
pub fn navigate(key: TreeKey, rows: &[NavRow], cursor: usize) -> Nav {
    let cursor = cursor.min(rows.len().saturating_sub(1));
    if let Some(next) = key.step(cursor, rows.len()) {
        return Nav::Cursor(next);
    }

    let Some(row) = rows.get(cursor) else { return Nav::None };
    match (key, row.open) {
        (TreeKey::Leave, _) => Nav::Leave,
        (TreeKey::Toggle, Some(open)) => Nav::SetOpen(cursor, !open),
        (TreeKey::Open, Some(false)) => Nav::SetOpen(cursor, true),
        (TreeKey::Open, Some(true)) => Nav::Cursor(TreeKey::Next.step(cursor, rows.len()).unwrap_or(cursor)),
        (TreeKey::Close, Some(true)) => Nav::SetOpen(cursor, false),
        (TreeKey::Close, _) => {
            rows[..cursor].iter().rposition(|above| above.depth < row.depth).map_or(Nav::None, Nav::Cursor)
        }
        _ => Nav::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `▼ Root` / `  Leaf` / `  ▶ Group` / `Other`
    const ROWS: [NavRow; 4] = [
        NavRow { depth: 0, open: Some(true) },
        NavRow { depth: 1, open: None },
        NavRow { depth: 1, open: Some(false) },
        NavRow { depth: 0, open: None },
    ];

    #[test]
    fn keys_move_through_the_rows() {
        assert_eq!(navigate(TreeKey::Next, &ROWS, 0), Nav::Cursor(1));
        assert_eq!(navigate(TreeKey::Next, &ROWS, 3), Nav::Cursor(3), "stops at the end");
        assert_eq!(navigate(TreeKey::Previous, &ROWS, 0), Nav::Cursor(0));
        assert_eq!(navigate(TreeKey::Last, &ROWS, 0), Nav::Cursor(3));
        assert_eq!(navigate(TreeKey::Next, &[], 0), Nav::Cursor(0));
    }

    #[test]
    fn keys_open_close_and_go_up_and_in() {
        assert_eq!(navigate(TreeKey::Open, &ROWS, 2), Nav::SetOpen(2, true), "`l` on a closed node");
        assert_eq!(navigate(TreeKey::Open, &ROWS, 0), Nav::Cursor(1), "`l` on an open one: into it");
        assert_eq!(navigate(TreeKey::Close, &ROWS, 0), Nav::SetOpen(0, false), "`h` on an open node");
        assert_eq!(navigate(TreeKey::Close, &ROWS, 1), Nav::Cursor(0), "`h` on a leaf: its parent");
        assert_eq!(navigate(TreeKey::Close, &ROWS, 2), Nav::Cursor(0), "`h` on a closed one: its parent");
        assert_eq!(navigate(TreeKey::Close, &ROWS, 3), Nav::None, "no parent");
        assert_eq!(navigate(TreeKey::Toggle, &ROWS, 0), Nav::SetOpen(0, false));
        assert_eq!(navigate(TreeKey::Toggle, &ROWS, 1), Nav::None, "a leaf");
        assert_eq!(navigate(TreeKey::Open, &ROWS, 1), Nav::None);
        assert_eq!(navigate(TreeKey::Leave, &ROWS, 1), Nav::Leave);
    }
}
