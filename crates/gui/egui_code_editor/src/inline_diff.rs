//! What the host changed compared to some older text, to show it inside the editor like Monaco's
//! inline diff:
//!
//! ```text
//! ↶ 12 <hkparam name="enable">true</hkparam>     red, read-only: the old line. Not part of the text
//!                                                `↶`: put this place back; what differs is darker
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
    /// The changed places, sorted by line.
    pub hunks: Vec<DiffHunk>,
}

/// One changed place: old lines replaced by new ones (either side may be empty).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DiffHunk {
    /// The new lines in the text (0-based). Empty: only a removal, which goes above `start`.
    pub new_lines: Range<usize>,
    /// The old lines, shown above the new ones.
    pub old_lines: Vec<String>,
    /// Per line pair (old line `n`, new line `n`), what differs in it. Empty unless there are as many
    /// old lines as new ones: otherwise it is not known which line became which.
    pub changes: Vec<CharChange>,
}

/// The chars that differ between an old line and its new line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharChange {
    pub old: Range<usize>,
    pub new: Range<usize>,
}

impl CharChange {
    /// What is left between the common start and the common end of two lines.
    ///
    /// e.g., `n>1<` and `n>25<`: `1` is chars 2..3, `25` is chars 2..4.
    #[must_use]
    pub fn between(old: &str, new: &str) -> Self {
        let (old_len, new_len) = (old.chars().count(), new.chars().count());
        let start = old.chars().zip(new.chars()).take_while(|(a, b)| a == b).count();
        let room = old_len.min(new_len) - start;
        let end = old.chars().rev().zip(new.chars().rev()).take(room).take_while(|(a, b)| a == b).count();
        Self { old: start..old_len - end, new: start..new_len - end }
    }
}

/// How far down each of `row_count` rows moves, in lines, followed by the total.
///
/// e.g., 3 rows, 2 old lines before row 1: `[0, 2, 2]`, total 2.
pub(crate) fn row_shifts(row_count: usize, hunks: &[DiffHunk]) -> (Vec<usize>, usize) {
    let mut shifts = Vec::with_capacity(row_count);
    let mut hunks = hunks.iter().peekable();
    let mut shift = 0;
    for row in 0..row_count {
        while let Some(hunk) = hunks.next_if(|hunk| hunk.new_lines.start <= row) {
            shift += hunk.old_lines.len();
        }
        shifts.push(shift);
    }
    (shifts, shift + hunks.map(|hunk| hunk.old_lines.len()).sum::<usize>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(before_line: usize, lines: usize) -> DiffHunk {
        DiffHunk { new_lines: before_line..before_line, old_lines: vec![String::new(); lines], changes: Vec::new() }
    }

    #[test]
    fn rows_move_below_the_old_lines() {
        assert_eq!(row_shifts(3, &[]), (vec![0, 0, 0], 0));
        assert_eq!(row_shifts(3, &[zone(1, 2)]), (vec![0, 2, 2], 2));
        assert_eq!(row_shifts(3, &[zone(0, 1), zone(2, 3)]), (vec![1, 1, 4], 4));
        // After the last line: no row moves, but the text gets taller.
        assert_eq!(row_shifts(2, &[zone(2, 3)]), (vec![0, 0], 3));
    }

    #[test]
    fn finds_what_differs_in_a_line() {
        let change = CharChange::between("<a>true</a>", "<a>false</a>");
        // (Both end in `e`, which is common.)
        assert_eq!(change, CharChange { old: 3..6, new: 3..7 });
        // Only added: nothing on the old side. The common start and end do not overlap.
        assert_eq!(CharChange::between("ab", "aab"), CharChange { old: 1..1, new: 1..2 });
        assert_eq!(CharChange::between("same", "same"), CharChange { old: 4..4, new: 4..4 });
    }
}
