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
//!
//! # Going through the changes
//! As VS Code's diff editor: [`InlineDiff::first_line`] where a diff opens, [`InlineDiff::step`] for
//! `▲ ▼` / Vim's `]c` `[c`, [`InlineDiff::position`] for the `2/5` beside them.

use std::ops::Range;

/// See the module docs. The host makes it when the text changes: [`InlineDiff::between`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InlineDiff {
    /// The changed places, sorted by line.
    pub hunks: Vec<DiffHunk>,
}

impl InlineDiff {
    /// The changes from `old` to `new` as they are written, with the line numbers of `new`. Lines
    /// as many on both sides are taken as each one edited into the other ([`DiffHunk::changes`]).
    ///
    /// e.g., `a\nb\n` -> `a\nc\n`: line 1 is new, and the old `b` goes above it.
    #[must_use]
    pub fn between(old: &str, new: &str) -> Self {
        let hunks = line_hunks(old, new);
        if hunks.is_empty() {
            return Self::default();
        }
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let hunks = hunks.into_iter().map(|(old_at, new_at)| {
            let old = &old_lines[old_at];
            let new = new_lines.get(new_at.clone()).unwrap_or_default();
            let pairs = old.iter().zip(new).filter(|_| old.len() == new.len());
            DiffHunk {
                new_lines: new_at,
                old_lines: old.iter().map(|line| (*line).to_owned()).collect(),
                changes: pairs.map(|(old, new)| CharChange::between(old, new)).collect(),
            }
        });
        Self { hunks: hunks.collect() }
    }

    /// The line of the first change, where a diff opens. e.g. changes at lines 3 and 9: `3`
    #[inline]
    #[must_use]
    pub fn first_line(&self) -> Option<usize> {
        self.hunks.first().map(|hunk| hunk.new_lines.start)
    }

    /// The line to go to for the previous or next change from `cursor_line`, around the ends.
    ///
    /// e.g., changes at lines 3 and 9: from line 5, next is 9 and previous is 3; from line 9, next is 3.
    #[must_use]
    pub fn step(&self, cursor_line: usize, forward: bool) -> Option<usize> {
        let hunks = &self.hunks;
        let found = if forward {
            hunks.iter().find(|hunk| hunk.new_lines.start > cursor_line).or_else(|| hunks.first())
        } else {
            hunks.iter().rfind(|hunk| hunk.last_line() < cursor_line).or_else(|| hunks.last())
        };
        found.map(|hunk| hunk.new_lines.start)
    }

    /// The change `cursor_line` is in, or else the next one, as `(index, count)`: what `2/5` shows.
    #[must_use]
    pub fn position(&self, cursor_line: usize) -> Option<(usize, usize)> {
        let count = self.hunks.len();
        let at = self.hunks.iter().position(|hunk| cursor_line <= hunk.last_line());
        (count > 0).then(|| (at.unwrap_or(count - 1), count))
    }

    /// `new` with change `index` of [`Self::between`] (of `old`, `new`) put back as in `old`: the `↶`.
    #[must_use]
    pub fn revert(old: &str, new: &str, index: usize) -> String {
        let Some((old_at, new_at)) = line_hunks(old, new).into_iter().nth(index) else {
            return new.to_owned();
        };
        let old_lines: Vec<&str> = old.split_inclusive('\n').collect();
        let new_lines: Vec<&str> = new.split_inclusive('\n').collect();
        let (before, after) = (new_lines.get(..new_at.start), new_lines.get(new_at.end..));
        [before.unwrap_or_default(), &old_lines[old_at], after.unwrap_or_default()].concat().concat()
    }
}

/// The changed lines of two texts, `(old lines, new lines)`, as git's histogram diff finds them.
fn line_hunks(old: &str, new: &str) -> Vec<(Range<usize>, Range<usize>)> {
    use imara_diff::{Algorithm, Diff, InternedInput};
    let input = InternedInput::new(old, new);
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_lines(&input);
    let range = |r: Range<u32>| r.start as usize..r.end as usize;
    diff.hunks().map(|hunk| (range(hunk.before), range(hunk.after))).collect()
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

impl DiffHunk {
    /// Its last line. Only a removal has none: it goes above its start.
    const fn last_line(&self) -> usize {
        let last = self.new_lines.end.saturating_sub(1);
        if last > self.new_lines.start { last } else { self.new_lines.start }
    }
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
    #[inline]
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

    #[test]
    fn it_goes_from_one_change_to_the_next() {
        let hunk = |new_lines| DiffHunk { new_lines, ..DiffHunk::default() };
        let diff = InlineDiff { hunks: vec![hunk(3..5), hunk(9..9), hunk(12..13)] };

        assert_eq!(diff.step(0, true), Some(3));
        assert_eq!(diff.step(3, true), Some(9), "from inside one, to the next one");
        assert_eq!(diff.step(9, true), Some(12));
        assert_eq!(diff.step(12, true), Some(3), "around the end");

        assert_eq!(diff.step(12, false), Some(9));
        assert_eq!(diff.step(9, false), Some(3));
        assert_eq!(diff.step(4, false), Some(12), "inside the first one: around the start");

        assert_eq!(diff.position(0), Some((0, 3)));
        assert_eq!(diff.position(4), Some((0, 3)));
        assert_eq!(diff.position(5), Some((1, 3)));
        assert_eq!(diff.position(99), Some((2, 3)));

        let none = InlineDiff::default();
        assert_eq!(none.step(0, true), None);
        assert_eq!(none.position(0), None);
    }

    #[test]
    fn a_changed_line_is_both_old_and_new() {
        let diff = InlineDiff::between("a\nb\nc\n", "a\nx\nc\n");
        let hunk = &diff.hunks[0];
        assert_eq!(hunk.new_lines, 1..2);
        assert_eq!(hunk.old_lines, ["b"]);
        assert_eq!(hunk.changes, [CharChange { old: 0..1, new: 0..1 }]);
    }

    #[test]
    fn additions_and_removals_alone() {
        let added = &InlineDiff::between("a\n", "a\nb\n").hunks[0];
        assert_eq!(added.new_lines, 1..2);
        assert!(added.old_lines.is_empty() && added.changes.is_empty());

        let removed = &InlineDiff::between("a\nb\nc\n", "a\nc\n").hunks[0];
        assert_eq!(removed.new_lines, 1..1);
        assert_eq!(removed.old_lines, ["b"]);

        assert_eq!(InlineDiff::between("a\n", "a\n"), InlineDiff::default());
    }

    #[test]
    fn places_can_be_reverted_one_by_one() {
        let (template, text) = ("a\nb\nc\nd\n", "a\nX\nc\nd\nY\n");
        assert_eq!(InlineDiff::revert(template, text, 0), "a\nb\nc\nd\nY\n");
        assert_eq!(InlineDiff::revert(template, text, 1), "a\nX\nc\nd\n");
        assert_eq!(InlineDiff::revert(template, text, 9), text);
    }
}
