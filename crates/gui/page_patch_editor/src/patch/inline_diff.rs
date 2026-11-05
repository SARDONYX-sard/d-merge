//! What the editor shows inside the text: the object compared to the template.
//!
//! This compares the text as it is written, unlike the patch preview: that one is normalized, so
//! its line numbers are not the editor's.

use egui_code_editor::{CharChange, DiffHunk, InlineDiff};

/// The changes from `template` to `text`, with the line numbers of `text`.
///
/// e.g., `a\nb\n` -> `a\nc\n`: line 1 is new, and the old `b` goes above it.
pub(super) fn changes(template: &str, text: &str) -> InlineDiff {
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

/// `text` with changed place `index` of [`changes`] put back as in `template`.
pub(super) fn revert(template: &str, text: &str, index: usize) -> String {
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
