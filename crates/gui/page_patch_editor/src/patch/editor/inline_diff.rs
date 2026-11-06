//! What the editor shows inside the text: the object compared to the template.
//!
//! This compares the text as it is written, unlike the patch preview: that one is normalized, so
//! its line numbers are not the editor's.

use egui_code_editor::{CharChange, DiffHunk, InlineDiff};

/// The changes from `template` to `text`, with the line numbers of `text`.
///
/// e.g., `a\nb\n` -> `a\nc\n`: line 1 is new, and the old `b` goes above it.
pub(in crate::patch) fn changes(template: &str, text: &str) -> InlineDiff {
    let hunks = nemesis_diff::line_changes(template, text);
    if hunks.is_empty() {
        return InlineDiff::default();
    }
    let template_lines: Vec<&str> = template.lines().collect();
    let text_lines: Vec<&str> = text.lines().collect();
    let hunks = hunks.into_iter().map(|hunk| {
        let old = &template_lines[hunk.old];
        let new = text_lines.get(hunk.new.clone()).unwrap_or_default();
        // As many lines on both sides: taken as each one edited into the other.
        let pairs = old.iter().zip(new).filter(|_| old.len() == new.len());
        DiffHunk {
            new_lines: hunk.new,
            old_lines: old.iter().map(|line| (*line).to_owned()).collect(),
            changes: pairs.map(|(old, new)| CharChange::between(old, new)).collect(),
        }
    });
    InlineDiff { hunks: hunks.collect() }
}

/// The changed place the cursor is at, or else the next one, as `(index, count)`: what `2/5` shows.
pub(in crate::patch) fn position(diff: &InlineDiff, cursor_line: usize) -> Option<(usize, usize)> {
    let count = diff.hunks.len();
    let at = diff.hunks.iter().position(|hunk| cursor_line <= last_line(hunk));
    (count > 0).then(|| (at.unwrap_or(count - 1), count))
}

/// The line to go to for the previous or next changed place from the cursor, around the ends.
///
/// e.g., changes at lines 3 and 9: from line 5, next is 9 and previous is 3; from line 9, next is 3.
pub(in crate::patch) fn step(diff: &InlineDiff, cursor_line: usize, forward: bool) -> Option<usize> {
    let hunks = &diff.hunks;
    let found = if forward {
        hunks.iter().find(|hunk| hunk.new_lines.start > cursor_line).or_else(|| hunks.first())
    } else {
        hunks.iter().rfind(|hunk| last_line(hunk) < cursor_line).or_else(|| hunks.last())
    };
    found.map(|hunk| hunk.new_lines.start)
}

/// The last line of a changed place. Only a removal has none: it goes above its start.
fn last_line(hunk: &DiffHunk) -> usize {
    hunk.new_lines.end.saturating_sub(1).max(hunk.new_lines.start)
}

/// `text` with changed place `index` of [`changes`] put back as in `template`.
pub(in crate::patch) fn revert(template: &str, text: &str, index: usize) -> String {
    let Some(hunk) = nemesis_diff::line_changes(template, text).into_iter().nth(index) else {
        return text.to_owned();
    };
    let template_lines: Vec<&str> = template.split_inclusive('\n').collect();
    let text_lines: Vec<&str> = text.split_inclusive('\n').collect();
    let (before, after) = (text_lines.get(..hunk.new.start), text_lines.get(hunk.new.end..));
    let old = &template_lines[hunk.old];
    let all = [before.unwrap_or_default(), old, after.unwrap_or_default()];
    all.concat().concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_goes_from_one_change_to_the_next() {
        let hunk = |new_lines| DiffHunk { new_lines, ..DiffHunk::default() };
        let diff = InlineDiff { hunks: vec![hunk(3..5), hunk(9..9), hunk(12..13)] };

        assert_eq!(step(&diff, 0, true), Some(3));
        assert_eq!(step(&diff, 3, true), Some(9), "from inside one, to the next one");
        assert_eq!(step(&diff, 9, true), Some(12));
        assert_eq!(step(&diff, 12, true), Some(3), "around the end");

        assert_eq!(step(&diff, 12, false), Some(9));
        assert_eq!(step(&diff, 9, false), Some(3));
        assert_eq!(step(&diff, 4, false), Some(12), "inside the first one: around the start");

        assert_eq!(position(&diff, 0), Some((0, 3)));
        assert_eq!(position(&diff, 4), Some((0, 3)));
        assert_eq!(position(&diff, 5), Some((1, 3)));
        assert_eq!(position(&diff, 99), Some((2, 3)));

        let none = InlineDiff::default();
        assert_eq!(step(&none, 0, true), None);
        assert_eq!(position(&none, 0), None);
    }

    #[test]
    fn a_changed_line_is_both_old_and_new() {
        let diff = changes("a\nb\nc\n", "a\nx\nc\n");
        let hunk = &diff.hunks[0];
        assert_eq!(hunk.new_lines, 1..2);
        assert_eq!(hunk.old_lines, ["b"]);
        assert_eq!(hunk.changes, [CharChange { old: 0..1, new: 0..1 }]);
    }

    #[test]
    fn additions_and_removals_alone() {
        let added = &changes("a\n", "a\nb\n").hunks[0];
        assert_eq!(added.new_lines, 1..2);
        assert!(added.old_lines.is_empty() && added.changes.is_empty());

        let removed = &changes("a\nb\nc\n", "a\nc\n").hunks[0];
        assert_eq!(removed.new_lines, 1..1);
        assert_eq!(removed.old_lines, ["b"]);

        assert_eq!(changes("a\n", "a\n"), InlineDiff::default());
    }

    #[test]
    fn places_can_be_reverted_one_by_one() {
        let (template, text) = ("a\nb\nc\nd\n", "a\nX\nc\nd\nY\n");
        assert_eq!(revert(template, text, 0), "a\nb\nc\nd\nY\n");
        assert_eq!(revert(template, text, 1), "a\nX\nc\nd\n");
        assert_eq!(revert(template, text, 9), text);
    }
}
