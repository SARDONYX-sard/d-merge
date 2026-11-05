//! What the editor shows inside the text: the object compared to the template.
//!
//! This compares the text as it is written, unlike the patch preview: that one is normalized, so
//! its line numbers are not the editor's.

use egui_code_editor::{InlineDiff, RemovedLines};

/// The changes from `template` to `text`, with the line numbers of `text`.
///
/// e.g., `a\nb\n` -> `a\nc\n`: line 1 is new, and the old `b` goes above it.
pub(super) fn changes(template: &str, text: &str) -> InlineDiff {
    let hunks = nemesis_diff::line_changes(template, text);
    if hunks.is_empty() {
        return InlineDiff::default();
    }
    let template_lines: Vec<&str> = template.lines().collect();
    let mut diff = InlineDiff::default();
    for hunk in hunks {
        if !hunk.new.is_empty() {
            diff.added.push(hunk.new.clone());
        }
        if !hunk.old.is_empty() {
            let lines = template_lines[hunk.old].iter().map(|line| (*line).to_owned()).collect();
            diff.removed.push(RemovedLines { before_line: hunk.new.start, lines });
        }
    }
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_line_is_both_old_and_new() {
        let diff = changes("a\nb\nc\n", "a\nx\nc\n");
        assert_eq!(diff.added.first(), Some(&(1..2)));
        assert_eq!(diff.removed, [RemovedLines { before_line: 1, lines: vec!["b".to_owned()] }]);
    }

    #[test]
    fn additions_and_removals_alone() {
        let added = changes("a\n", "a\nb\n");
        assert_eq!(added.added.first(), Some(&(1..2)));
        assert!(added.removed.is_empty());

        let removed = changes("a\nb\nc\n", "a\nc\n");
        assert!(removed.added.is_empty());
        assert_eq!(removed.removed, [RemovedLines { before_line: 1, lines: vec!["b".to_owned()] }]);

        assert_eq!(changes("a\n", "a\n"), InlineDiff::default());
    }
}
