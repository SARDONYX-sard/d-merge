//! What the host changed compared to some older text, to show it inside the editor like Monaco's
//! inline diff:
//!
//! ```text
//! 12   <hkparam name="enable">true</hkparam>     red, read-only: the old line. Not part of the text
//! 12 + <hkparam name="enable">false</hkparam>    green tint: a new line, edited as usual
//! 13   <hkparam name="userData">1</hkparam>
//! ```
//!
//! # How the old lines fit in
//! They are not in the text, so cursor positions stay right. After the layout, the rows below them
//! are moved down ([`row_shifts`]), which leaves a gap they are painted in. egui reads a row's
//! place from the row, so clicks and cursor moves work across the gaps.

use std::ops::Range;

/// See the module docs. The host computes it when the text changes. e.g., from a line diff.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InlineDiff {
    /// Lines of the text that are new (0-based), sorted.
    pub added: Vec<Range<usize>>,
    /// Old lines to show, sorted by [`RemovedLines::before_line`].
    pub removed: Vec<RemovedLines>,
}

/// Lines that are gone, shown above a line of the text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RemovedLines {
    /// The line of the text they go above. One past the last line: after the text.
    pub before_line: usize,
    pub lines: Vec<String>,
}

/// How far down each of `row_count` rows moves, in lines, followed by the total.
///
/// e.g., 3 rows, 2 old lines before row 1: `[0, 2, 2]`, total 2.
pub(crate) fn row_shifts(row_count: usize, removed: &[RemovedLines]) -> (Vec<usize>, usize) {
    let mut shifts = Vec::with_capacity(row_count);
    let mut zones = removed.iter().peekable();
    let mut shift = 0;
    for row in 0..row_count {
        while let Some(zone) = zones.next_if(|zone| zone.before_line <= row) {
            shift += zone.lines.len();
        }
        shifts.push(shift);
    }
    (shifts, shift + zones.map(|zone| zone.lines.len()).sum::<usize>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(before_line: usize, lines: usize) -> RemovedLines {
        RemovedLines { before_line, lines: vec![String::new(); lines] }
    }

    #[test]
    fn rows_move_below_the_old_lines() {
        assert_eq!(row_shifts(3, &[]), (vec![0, 0, 0], 0));
        assert_eq!(row_shifts(3, &[zone(1, 2)]), (vec![0, 2, 2], 2));
        assert_eq!(row_shifts(3, &[zone(0, 1), zone(2, 3)]), (vec![1, 1, 4], 4));
        // After the last line: no row moves, but the text gets taller.
        assert_eq!(row_shifts(2, &[zone(2, 3)]), (vec![0, 0], 3));
    }
}
