//! Hover popup: diagnostics + [`Language::hover`] docs, VS Code style.
//!
//! An egui tooltip follows the pointer, so links in it can never be reached.
//! This popup is **pinned under the hovered char** instead:
//! - opens after the pointer rests on a char for [`HOVER_DELAY`]
//! - stays open while the pointer is on the text or inside the popup (links are clickable)
//! - closes when the pointer leaves both, on any key press, or on a click outside it
//! - Vim `K` opens it at the text cursor (see [`HoverPopup::open_at`]); `K` again moves the
//!   keyboard into it: `Tab` / `j` / `k` go through the links, `Enter` opens one,
//!   `Esc` / `q` close it and return to the editor

use egui::{Galley, Pos2, Rect};

use crate::{
    language::{Diagnostic, Language, Severity},
    text_pos::LinePos,
};

/// Seconds the pointer must rest on a char before the popup opens (or moves to it).
pub(crate) const HOVER_DELAY: f64 = 0.3;

/// Slack (in points) around the popup, so a small gap between the text and the popup
/// does not close it while the pointer travels into it.
pub(crate) const POPUP_MARGIN: f32 = 8.0;

/// Per-editor hover state (part of [`crate::CodeEditorState`]).
#[derive(Debug, Default)]
pub(crate) struct HoverPopup {
    /// Char index the pointer rests on, and since when (seconds).
    pub(crate) resting: Option<(usize, f64)>,
    /// Pointer position when a key was pressed (or the popup was clicked away): no hover popup
    /// opens until the pointer moves from there, so it does not get in the way while typing or
    /// moving with Vim keys (like VS Code).
    pub(crate) suppressed_at: Option<Pos2>,
    pub(crate) open: Option<Open>,
}

#[derive(Debug)]
pub(crate) struct Open {
    /// Char index the popup describes.
    pub(crate) index: usize,
    /// Screen rect of that char (zero width, row height); the popup is placed below it.
    pub(crate) anchor: Rect,
    pub(crate) content: Content,
    /// Popup rect drawn last frame (to know whether the pointer is inside).
    pub(crate) rect: Option<Rect>,
    /// Opened by keyboard (Vim `K`): the pointer does not move or close it.
    pub(crate) pinned: bool,
    /// The keyboard is in the popup (Vim `K` twice): keys go to its links, not to the editor.
    pub(crate) focused: bool,
    /// Focus the first link on the next draw.
    pub(crate) focus_first_link: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Content {
    pub(crate) problems: Vec<(Severity, String)>,
    pub(crate) docs: Option<String>,
}

/// What the popup needs from the editor this frame.
pub(crate) struct HoverCtx<'a> {
    pub(crate) id: egui::Id,
    pub(crate) response: &'a egui::Response,
    pub(crate) galley: &'a Galley,
    pub(crate) galley_pos: Pos2,
    pub(crate) text: &'a str,
    pub(crate) language: &'a dyn Language,
    pub(crate) diagnostics: &'a [Diagnostic],
    /// Show [`Language::hover`] docs (diagnostics are shown regardless).
    pub(crate) with_docs: bool,
}

/// Diagnostics at `index` + the language's docs, or `None` if there is nothing to show.
pub(crate) fn content(index: usize, ctx: &HoverCtx<'_>) -> Option<Content> {
    let docs = ctx.with_docs.then_some(ctx.language);
    content_at(ctx.text, index, ctx.diagnostics, docs)
}

/// See [`content`]. `language`: `None` shows the diagnostics only.
fn content_at(
    text: &str,
    index: usize,
    diagnostics: &[Diagnostic],
    language: Option<&dyn Language>,
) -> Option<Content> {
    let pos = LinePos::from_char_index(text, index);
    let problems: Vec<(Severity, String)> = diagnostics
        .iter()
        .filter(|d| d.line == pos.line && d.columns.start <= pos.column && pos.column <= d.columns.end)
        .map(|d| (d.severity, d.message.clone()))
        .collect();
    let docs = language.and_then(|language| language.hover_at(text, pos.line, pos.column));

    (!problems.is_empty() || docs.is_some()).then_some(Content { problems, docs })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Docs;
    impl Language for Docs {
        fn hover(&self, line: &str, _column: usize) -> Option<String> {
            Some(line.to_uppercase())
        }
    }

    #[test]
    fn content_has_problems_on_that_line_and_docs() {
        let problem = Diagnostic { line: 1, columns: 0..2, severity: Severity::Error, message: "bad".to_owned() };
        let problems = [problem];

        // `b` of line 1: on the problem, and the language's docs of that line.
        let content = content_at("aa\nbb", 3, &problems, Some(&Docs)).unwrap();
        assert_eq!(content.problems, [(Severity::Error, "bad".to_owned())]);
        assert_eq!(content.docs.as_deref(), Some("BB"));

        // Line 0, docs off: nothing to show.
        assert!(content_at("aa\nbb", 0, &problems, None).is_none());
    }
}
