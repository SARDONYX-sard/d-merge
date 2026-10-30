//! Hover popup: diagnostics + [`Language::hover`] docs, VS Code style.
//!
//! An egui tooltip follows the pointer, so links in it can never be reached.
//! This popup is **pinned under the hovered char** instead:
//! - opens after the pointer rests on a char for [`HOVER_DELAY`]
//! - stays open while the pointer is on the text or inside the popup (links are clickable)
//! - closes when the pointer leaves both, on any key press, or on a click outside it
//! - Vim `K` opens it at the text cursor (see [`HoverPopup::open_at`])

use egui::{Galley, Pos2, Rect, text::CCursor};

use crate::{
    language::{Diagnostic, Language, Severity},
    text_pos::LinePos,
};

/// Seconds the pointer must rest on a char before the popup opens (or moves to it).
const HOVER_DELAY: f64 = 0.3;

/// Slack (in points) around the popup, so a small gap between the text and the popup
/// does not close it while the pointer travels into it.
const POPUP_MARGIN: f32 = 8.0;

/// Per-editor hover state (part of [`crate::CodeEditorState`]).
#[derive(Debug, Default)]
pub(crate) struct HoverPopup {
    /// Char index the pointer rests on, and since when (seconds).
    resting: Option<(usize, f64)>,
    /// Closed by a key press / click: do not reopen until the pointer moves to another char.
    dismissed: Option<usize>,
    open: Option<Open>,
}

#[derive(Debug)]
struct Open {
    /// Char index the popup describes.
    index: usize,
    /// Screen rect of that char (zero width, row height); the popup is placed below it.
    anchor: Rect,
    content: Content,
    /// Popup rect drawn last frame (to know whether the pointer is inside).
    rect: Option<Rect>,
    /// Opened by keyboard (Vim `K`): the pointer does not move or close it.
    pinned: bool,
}

#[derive(Debug, PartialEq)]
struct Content {
    problems: Vec<(Severity, String)>,
    docs: Option<String>,
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

impl HoverPopup {
    /// Closes the popup on a key press or a click outside it.
    ///
    /// Call at the start of the frame, **before** the keymap swallows the key events.
    pub(crate) fn close_on_input(&mut self, ui: &egui::Ui) {
        let Some(open) = &self.open else { return };
        let (key_pressed, click_pos) = ui.input(|i| {
            let key = i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. }));
            let click = i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten();
            (key, click)
        });
        let clicked_outside =
            click_pos.is_some_and(|pos| !open.rect.is_some_and(|rect| rect.contains(pos)));
        if key_pressed || clicked_outside {
            self.dismissed = Some(open.index);
            self.open = None;
            self.resting = None;
        }
    }

    /// Opens the popup for the char at `index` right away (e.g., Vim `K` at the cursor).
    pub(crate) fn open_at(&mut self, index: usize, ctx: &HoverCtx<'_>) {
        let anchor = char_rect(ctx.galley, ctx.galley_pos, index);
        self.open = content(index, ctx).map(|content| Open {
            index,
            anchor,
            content,
            rect: None,
            pinned: true,
        });
        self.dismissed = None;
    }

    /// Tracks the pointer and draws the popup.
    pub(crate) fn ui(&mut self, ui: &egui::Ui, ctx: &HoverCtx<'_>) {
        let now = ui.input(|i| i.time);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let over_popup = self
            .open
            .as_ref()
            .and_then(|o| o.rect)
            .is_some_and(|rect| pointer.is_some_and(|p| rect.expand(POPUP_MARGIN).contains(p)));

        // `hover_pos` is `None` while the pointer is over the popup (it is on a higher layer).
        let target = ctx.response.hover_pos().and_then(|p| char_at(ctx.galley, ctx.galley_pos, p));

        let pinned = self.open.as_ref().is_some_and(|o| o.pinned);
        match target {
            Some(index) if !over_popup && !pinned => self.on_char(index, now, ui, ctx),
            Some(_) => {}
            None => {
                self.resting = None;
                self.dismissed = None;
                if !over_popup && !pinned && self.open.is_some() && pointer.is_some() {
                    self.open = None; // pointer left both the text and the popup
                }
            }
        }

        if let Some(open) = &mut self.open {
            // Re-anchor every frame so the popup follows its char when the text scrolls.
            open.anchor = char_rect(ctx.galley, ctx.galley_pos, open.index);
            open.rect = Some(show(ui.ctx(), ctx.id, open));
        }
    }

    /// The pointer is on the char at `index`: wait for the delay, then open / move the popup.
    fn on_char(&mut self, index: usize, now: f64, ui: &egui::Ui, ctx: &HoverCtx<'_>) {
        if self.dismissed.is_some_and(|i| i != index) {
            self.dismissed = None;
        }
        let since = match self.resting {
            Some((i, since)) if i == index => since,
            _ => {
                self.resting = Some((index, now));
                now
            }
        };

        let waited = now - since;
        if waited < HOVER_DELAY {
            ui.ctx().request_repaint_after_secs((HOVER_DELAY - waited) as f32);
            return;
        }
        if self.dismissed.is_some() || self.open.as_ref().is_some_and(|o| o.index == index) {
            return;
        }

        match content(index, ctx) {
            // Same text for the next char of the same word: keep the popup where it is.
            Some(new) if self.open.as_ref().is_some_and(|o| o.content == new) => {}
            Some(content) => {
                let anchor = char_rect(ctx.galley, ctx.galley_pos, index);
                self.open = Some(Open { index, anchor, content, rect: None, pinned: false });
            }
            None => self.open = None,
        }
    }
}

/// Diagnostics at `index` + the language's docs, or `None` if there is nothing to show.
fn content(index: usize, ctx: &HoverCtx<'_>) -> Option<Content> {
    let pos = LinePos::from_char_index(ctx.text, index);
    let line = ctx.text.split('\n').nth(pos.line).unwrap_or_default();

    let problems: Vec<(Severity, String)> = ctx
        .diagnostics
        .iter()
        .filter(|d| {
            d.line == pos.line && d.columns.start <= pos.column && pos.column <= d.columns.end
        })
        .map(|d| (d.severity, d.message.clone()))
        .collect();
    let docs = if ctx.with_docs { ctx.language.hover(line, pos.column) } else { None };

    (!problems.is_empty() || docs.is_some()).then_some(Content { problems, docs })
}

/// Draws the popup below its anchor and returns its rect.
fn show(ctx: &egui::Context, id: egui::Id, open: &Open) -> Rect {
    egui::Area::new(id.with("hover_popup"))
        .order(egui::Order::Tooltip)
        .fixed_pos(open.anchor.left_bottom())
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(480.0);
                let Content { problems, docs } = &open.content;
                for (severity, message) in problems {
                    ui.colored_label(severity.color(ui.visuals()), message);
                }
                if let Some(docs) = docs {
                    if !problems.is_empty() {
                        ui.separator();
                    }
                    egui_widgets::markdown::markdown(ui, docs);
                }
            });
        })
        .response
        .rect
}

/// Char index under `pointer`, or `None` in the empty area right of a line.
fn char_at(galley: &Galley, galley_pos: Pos2, pointer: Pos2) -> Option<usize> {
    let cursor = galley.cursor_from_pos(pointer - galley_pos);
    let glyph = galley.pos_from_cursor(cursor).translate(galley_pos.to_vec2());
    let near_glyph = (pointer.x - glyph.left()).abs() <= 40.0
        && glyph.expand2(egui::vec2(0.0, 2.0)).y_range().contains(pointer.y);
    near_glyph.then_some(cursor.index.0)
}

/// Screen rect of the char at `index` (zero width, row height).
fn char_rect(galley: &Galley, galley_pos: Pos2, index: usize) -> Rect {
    galley.pos_from_cursor(CCursor::new(index)).translate(galley_pos.to_vec2())
}
