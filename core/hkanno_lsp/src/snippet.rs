//! Minimal snippet syntax used by completion items.
//!
//! Only `${N:placeholder}` is supported (the subset the hkanno completions use).
//! The GUI inserts [`Expanded::text`] and selects the first placeholder so the user can type over it.
//!
//! # Example
//! ```
//! let expanded = hkanno_lsp::snippet::expand("animrotation ${1:0}");
//! assert_eq!(expanded.text, "animrotation 0");
//! // The `0` is pre-selected.
//! assert_eq!(expanded.first_placeholder, Some(13..14));
//! ```

use std::ops::Range;

/// A snippet turned into plain text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expanded {
    /// Text to insert.
    pub text: String,
    /// `char` range (relative to the start of `text`) of the `${1:...}` placeholder.
    /// Falls back to the first placeholder of any number.
    pub first_placeholder: Option<Range<usize>>,
}

/// Replaces every `${N:placeholder}` with `placeholder`.
pub fn expand(snippet: &str) -> Expanded {
    let mut text = String::with_capacity(snippet.len());
    let mut text_chars = 0; // `text.chars().count()`, tracked incrementally.
    let mut first: Option<(u32, Range<usize>)> = None;

    let mut rest = snippet;
    while let Some(open) = rest.find("${") {
        let (before, after_open) = (&rest[..open], &rest[open + 2..]);
        text.push_str(before);
        text_chars += before.chars().count();

        // `N:placeholder}`
        let Some(close) = after_open.find('}') else {
            // Not a snippet after all: keep the remaining text as-is.
            rest = &rest[open..];
            break;
        };
        let body = &after_open[..close];
        let (number, placeholder) = body.split_once(':').unwrap_or((body, ""));
        let number = number.parse::<u32>().unwrap_or(u32::MAX);

        let start = text_chars;
        text.push_str(placeholder);
        text_chars += placeholder.chars().count();

        // Prefer `${1:...}`, otherwise the smallest number.
        if first.as_ref().is_none_or(|(n, _)| number < *n) {
            first = Some((number, start..text_chars));
        }

        rest = &after_open[close + 1..];
    }
    text.push_str(rest);

    Expanded { text, first_placeholder: first.map(|(_, range)| range) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_placeholders() {
        let e = expand("SGVB|${1:graphVariable}|${2:bool}");
        assert_eq!(e.text, "SGVB|graphVariable|bool");
        assert_eq!(e.first_placeholder, Some(5..18));
    }

    #[test]
    fn no_placeholder() {
        assert_eq!(expand(".").first_placeholder, None);
    }
}
