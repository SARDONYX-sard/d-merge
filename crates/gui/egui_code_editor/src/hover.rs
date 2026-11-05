//! Hover popup: diagnostics + [`Language::hover`] docs, VS Code style.
//!
//! An egui tooltip follows the pointer, so links in it can never be reached.
//! This popup is **pinned under the hovered char** instead:
//! - opens after the pointer rests on a char for [`HOVER_DELAY`]
//! - stays open while the pointer is on the text or inside the popup (links are clickable)
//! - closes when the pointer leaves both, on any key press, or on a click outside it
//! - scrolls when its text is taller than [`MAX_HEIGHT`]
//! - Vim `K` opens it at the text cursor (see [`HoverPopup::open_at`]); `K` again moves the
//!   keyboard into it if there is anything to do there: `j` / `k` scroll, `Tab` / `Shift+Tab` go
//!   through the links, `Enter` opens one, `Esc` / `q` close it and return to the editor

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

/// The popup scrolls above this height, so that a long text does not cover the editor.
pub(crate) const MAX_HEIGHT: f32 = 320.0;

/// How far `j` / `k` scroll the popup: about three lines.
pub(crate) const SCROLL_STEP: f32 = 48.0;

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
    /// The keyboard is in the popup (Vim `K` twice): keys scroll it and go to its links, not to the
    /// editor.
    pub(crate) focused: bool,
    /// How many links the last draw had.
    pub(crate) links: usize,
    /// The link the keyboard is on. `None`: on the popup itself.
    pub(crate) link: Option<usize>,
    /// [`Self::link`] changed: bring it into view on the next draw.
    pub(crate) follow_link: bool,
    /// The last draw had more text than fits.
    pub(crate) can_scroll: bool,
    /// How far it was scrolled at the last draw.
    pub(crate) scroll_offset: f32,
    /// Scroll here on the next draw. Otherwise the mouse wheel decides.
    pub(crate) scroll_to: Option<f32>,
}

impl Open {
    pub(crate) const fn new(index: usize, anchor: Rect, content: Content, pinned: bool) -> Self {
        Self {
            index,
            anchor,
            content,
            rect: None,
            pinned,
            focused: false,
            links: 0,
            link: None,
            follow_link: false,
            can_scroll: false,
            scroll_offset: 0.0,
            // A popup that was open at the same place before must not start where that one ended.
            scroll_to: Some(0.0),
        }
    }

    /// Is there anything for the keyboard to do in the popup? If not, it stays in the editor.
    pub(crate) const fn takes_keyboard(&self) -> bool {
        self.links > 0 || self.can_scroll
    }

    /// `Tab` (`forward`) / `Shift+Tab`.
    pub(crate) const fn step_link(&mut self, forward: bool) {
        self.link = next_link(self.link, self.links, forward);
        self.follow_link = true;
    }

    /// `j` (positive) / `k` (negative). The scroll area keeps it within the text.
    pub(crate) const fn scroll_by(&mut self, delta: f32) {
        self.scroll_to = Some(self.scroll_offset + delta);
    }
}

/// The link after `current` of `links`, around the ends. From no link: the first, or the last going
/// backward.
///
/// e.g., of 3 links: `None` -> `Some(0)`, `Some(2)` -> `Some(0)`; backward, `Some(0)` -> `Some(2)`
pub(crate) const fn next_link(current: Option<usize>, links: usize, forward: bool) -> Option<usize> {
    if links == 0 {
        return None;
    }
    Some(match (current, forward) {
        (None, true) => 0,
        (None, false) => links - 1,
        (Some(at), true) => (at + 1) % links,
        (Some(at), false) => (at + links - 1) % links,
    })
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

    #[test]
    fn links_go_around() {
        assert_eq!(next_link(None, 3, true), Some(0));
        assert_eq!(next_link(None, 3, false), Some(2));
        assert_eq!(next_link(Some(0), 3, true), Some(1));
        assert_eq!(next_link(Some(2), 3, true), Some(0));
        assert_eq!(next_link(Some(0), 3, false), Some(2));
        assert_eq!(next_link(None, 0, true), None);
    }

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
