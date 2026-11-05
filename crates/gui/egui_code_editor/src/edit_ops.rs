//! Text edits by char range, for the right-click menu. Ranges are in chars, like egui's cursors.

use std::ops::Range;

use crate::text_pos::char_to_byte;

/// The text of a non-empty selection.
///
/// e.g., `日本語`, `1..2` -> `本`; an empty range -> `None` (nothing to copy).
pub(crate) fn selected_text<'a>(text: &'a str, range: &Range<usize>) -> Option<&'a str> {
    (!range.is_empty()).then(|| &text[char_to_byte(text, range.start)..char_to_byte(text, range.end)])
}

/// Replaces the chars of `range` with `new_text`. Returns the char index right after it, for the
/// cursor.
///
/// e.g., `abc`, `1..2`, `XY` -> `aXYc`, returns 3.
pub(crate) fn replace_chars(text: &mut String, range: &Range<usize>, new_text: &str) -> usize {
    let bytes = char_to_byte(text, range.start)..char_to_byte(text, range.end);
    text.replace_range(bytes, new_text);
    range.start + new_text.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_by_chars() {
        assert_eq!(selected_text("日本語", &(1..2)), Some("本"));
        assert_eq!(selected_text("abc", &(1..1)), None);
    }

    #[test]
    fn replaces_by_chars() {
        let mut text = "日本語".to_owned();
        assert_eq!(replace_chars(&mut text, &(1..2), "XY"), 3);
        assert_eq!(text, "日XY語");

        // An empty range inserts; empty text deletes.
        assert_eq!(replace_chars(&mut text, &(0..0), "a"), 1);
        assert_eq!(replace_chars(&mut text, &(0..1), ""), 0);
        assert_eq!(text, "日XY語");
    }
}
