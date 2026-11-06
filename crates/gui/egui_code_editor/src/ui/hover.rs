//! Tracks the pointer and draws the hover popup. The state and its content are in [`crate::hover`].

use egui::{Galley, Pos2, Rect, text::CCursor};

use crate::hover::{Content, HOVER_DELAY, HoverCtx, HoverPopup, Open, POPUP_MARGIN, SCROLL_STEP, content, max_height};

/// The width of the popup's border while the keyboard is in it.
const FOCUSED_BORDER: f32 = 2.0;

/// The space around the text of the popup: as VS Code's hover (`padding: 4px 8px`).
const TEXT_MARGIN: egui::Margin = egui::Margin::symmetric(8, 4);

impl HoverPopup {
    /// Handles the keys meant for the popup, and closes it on other keys or a click outside it.
    ///
    /// Call at the start of the frame, **before** the keymap swallows the key events.
    /// `editor_id`: gets the keyboard focus back when a focused popup closes.
    pub(crate) fn handle_input(&mut self, ui: &egui::Ui, editor_id: egui::Id) {
        let (key_pressed, click_pos, pointer) = ui.input(|i| {
            let key = i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. }));
            let click = i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten();
            (key, click, i.pointer.hover_pos())
        });
        if key_pressed {
            self.suppressed_at = Some(pointer.unwrap_or(Pos2::ZERO));
            self.resting = None;
        }

        let Some(open) = &mut self.open else { return };

        if open.focused {
            focused_keys(ui, open, editor_id);
            if !open.focused {
                self.open = None;
            }
            return;
        }

        // Vim `K` on an open `K` popup: move the keyboard into it (like Neovim's hover window). With
        // nothing to scroll or follow it stays in the editor, so the next key closes the popup as
        // usual.
        if open.pinned && take_shift_k(ui) {
            open.focused = open.takes_keyboard();
            return;
        }

        // Any other key (e.g., Vim `j`) or a click elsewhere closes it.
        let clicked_outside = click_pos.is_some_and(|pos| !open.rect.is_some_and(|rect| rect.contains(pos)));
        if key_pressed || clicked_outside {
            if clicked_outside {
                self.suppressed_at = click_pos;
            }
            self.open = None;
            self.resting = None;
        }
    }

    /// Opens the popup at `open_at` if asked to, tracks the pointer and draws.
    pub(crate) fn show(&mut self, ui: &egui::Ui, ctx: &HoverCtx<'_>, open_at: Option<usize>) {
        if let Some(index) = open_at {
            self.open_at(index, ctx);
        }
        self.ui(ui, ctx);
    }

    /// Opens the popup for the char at `index` right away (e.g., Vim `K` at the cursor).
    pub(crate) fn open_at(&mut self, index: usize, ctx: &HoverCtx<'_>) {
        let anchor = char_rect(ctx.galley, ctx.galley_pos, index);
        self.open = content(index, ctx).map(|content| Open::new(index, anchor, content, true));
    }

    /// Tracks the pointer and draws the popup.
    pub(crate) fn ui(&mut self, ui: &egui::Ui, ctx: &HoverCtx<'_>) {
        let now = ui.input(|i| i.time);
        let pointer = ui.input(|i| i.pointer.hover_pos());
        // A button is down: the pointer is selecting, not asking. Where it stops is not a question
        // either, until it moves on.
        if ui.input(|i| i.pointer.any_down()) {
            self.resting = None;
            self.suppressed_at = pointer;
        }
        let over_popup = self
            .open
            .as_ref()
            .and_then(|o| o.rect)
            .is_some_and(|rect| pointer.is_some_and(|p| rect.expand(POPUP_MARGIN).contains(p)));

        // `hover_pos` is `None` while the pointer is over the popup (it is on a higher layer).
        let target = ctx.response.hover_pos().and_then(|p| char_at(ctx.galley, ctx.galley_pos, p));

        let pinned = self.open.as_ref().is_some_and(|o| o.pinned);
        match target {
            Some(index) if !over_popup && !pinned => self.on_char(index, pointer, now, ui, ctx),
            Some(_) => {}
            None => {
                self.resting = None;
                if !over_popup && !pinned && self.open.is_some() && pointer.is_some() {
                    self.open = None; // pointer left both the text and the popup
                }
            }
        }

        if let Some(open) = &mut self.open {
            // Re-anchor every frame so the popup follows its char when the text scrolls.
            open.anchor = char_rect(ctx.galley, ctx.galley_pos, open.index);
            open.rect = Some(show(ui.ctx(), ctx.id, open, max_height(ctx.response.rect.height())));
        }
    }

    /// The pointer is on the char at `index`: wait for the delay, then open / move the popup.
    fn on_char(&mut self, index: usize, pointer: Option<Pos2>, now: f64, ui: &egui::Ui, ctx: &HoverCtx<'_>) {
        if let Some(at) = self.suppressed_at {
            if pointer.is_some_and(|p| p.distance(at) < 3.0) {
                return;
            }
            self.suppressed_at = None;
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
        if self.open.as_ref().is_some_and(|o| o.index == index) {
            return;
        }

        match content(index, ctx) {
            // Same text for the next char of the same word: keep the popup where it is.
            Some(new) if self.open.as_ref().is_some_and(|o| o.content == new) => {}
            Some(content) => {
                let anchor = char_rect(ctx.galley, ctx.galley_pos, index);
                self.open = Some(Open::new(index, anchor, content, false));
            }
            None => self.open = None,
        }
    }
}

/// Keys while the keyboard is in the popup. Sets `open.focused = false` to close it.
fn focused_keys(ui: &egui::Ui, open: &mut Open, editor_id: egui::Id) {
    use egui::{Key, Modifiers};

    let keys = ui.input_mut(|i| {
        let close = i.consume_key(Modifiers::NONE, Key::Escape) || i.consume_key(Modifiers::NONE, Key::Q);
        let down = i.consume_key(Modifiers::NONE, Key::J) || i.consume_key(Modifiers::NONE, Key::ArrowDown);
        let up = i.consume_key(Modifiers::NONE, Key::K) || i.consume_key(Modifiers::NONE, Key::ArrowUp);
        let next = i.consume_key(Modifiers::NONE, Key::Tab);
        let previous = i.consume_key(Modifiers::SHIFT, Key::Tab);
        let click = i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten();
        let outside = click.is_some_and(|pos| !open.rect.is_some_and(|rect| rect.contains(pos)));
        // What these keys typed must not reach the editor, which gets the keyboard back on `q`.
        i.events.retain(|event| !matches!(event, egui::Event::Text(_)));
        (close || outside, down, up, next, previous)
    });
    let (close, down, up, next, previous) = keys;

    if close {
        open.focused = false;
        ui.memory_mut(|m| m.request_focus(editor_id));
    } else if down {
        open.scroll_by(SCROLL_STEP);
    } else if up {
        open.scroll_by(-SCROLL_STEP);
    } else if next {
        open.step_link(true);
    } else if previous {
        open.step_link(false);
    }
}

/// Takes a `Shift+K` press (both its key and its text event) out of the input.
fn take_shift_k(ui: &egui::Ui) -> bool {
    ui.input_mut(|i| {
        if !i.consume_key(egui::Modifiers::SHIFT, egui::Key::K) {
            return false;
        }
        i.events.retain(|e| !matches!(e, egui::Event::Text(t) if t == "K"));
        true
    })
}

/// Draws the popup below its anchor and returns its rect.
fn show(ctx: &egui::Context, id: egui::Id, open: &mut Open, max_height: f32) -> Rect {
    egui::Area::new(id.with("hover_popup"))
        .order(egui::Order::Tooltip)
        .fixed_pos(open.anchor.left_bottom())
        .constrain(true)
        // An area can't grow beyond its last size, which is that of the last popup: measure new text.
        // (Not drawn for this one frame, in which egui asks for the next.)
        .sizing_pass(open.rect.is_none())
        .show(ctx, |ui| {
            popup_frame(ui.style(), open.focused).show(ui, |ui| {
                // Has the keyboard while no link does, so that it leaves the editor.
                let popup = ui.allocate_response(egui::Vec2::ZERO, egui::Sense::focusable_noninteractive());
                let mut area = egui::ScrollArea::vertical().id_salt("hover_scroll").max_height(max_height);
                if let Some(offset) = open.scroll_to.take() {
                    area = area.vertical_scroll_offset(offset);
                }
                let scrolled = area.show(ui, |ui| {
                    ui.set_max_width(480.0);
                    let links = popup_text(ui, &open.content);
                    if open.focused {
                        let link = open.link.and_then(|link| links.get(link));
                        hold_keyboard(ui, link.unwrap_or(&popup));
                        if open.follow_link
                            && let Some(link) = link
                        {
                            link.scroll_to_me(None);
                        }
                    }
                    open.follow_link = false;
                    links.len()
                });
                open.links = scrolled.inner;
                open.can_scroll = scrolled.content_size.y > scrolled.inner_rect.height();
                open.scroll_offset = scrolled.state.clamped_offset().y;
            });
        })
        .response
        .rect
}

/// The popup's frame. With the keyboard in the popup its border stands out: keys go there, not to the
/// editor.
fn popup_frame(style: &egui::Style, focused: bool) -> egui::Frame {
    let frame = egui::Frame::popup(style).inner_margin(TEXT_MARGIN);
    if focused { frame.stroke(egui::Stroke::new(FOCUSED_BORDER, style.visuals.selection.bg_fill)) } else { frame }
}

/// The problems, then the docs. Returns the links in the docs.
fn popup_text(ui: &mut egui::Ui, content: &Content) -> Vec<egui::Response> {
    let Content { problems, docs } = content;
    for (severity, message) in problems {
        ui.colored_label(severity.color(ui.visuals()), message);
    }
    let Some(docs) = docs else { return Vec::new() };
    if !problems.is_empty() {
        ui.separator();
    }
    egui_widgets::markdown::markdown(ui, docs)
}

/// Keeps the keyboard on `widget`, with egui leaving `Tab` and the arrows to [`focused_keys`]: on its
/// own, it would move them to any widget of the window.
fn hold_keyboard(ui: &egui::Ui, widget: &egui::Response) {
    // (Asking again would reset the filter below.)
    if !widget.has_focus() {
        widget.request_focus();
    }
    let filter = egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true };
    ui.memory_mut(|m| m.set_focus_lock_filter(widget.id, filter));
}

/// Char index under `pointer`, or `None` in the empty area right of a line.
fn char_at(galley: &Galley, galley_pos: Pos2, pointer: Pos2) -> Option<usize> {
    let cursor = galley.cursor_from_pos(pointer - galley_pos);
    let glyph = galley.pos_from_cursor(cursor).translate(galley_pos.to_vec2());
    let near_glyph =
        (pointer.x - glyph.left()).abs() <= 40.0 && glyph.expand2(egui::vec2(0.0, 2.0)).y_range().contains(pointer.y);
    near_glyph.then_some(cursor.index.0)
}

/// Screen rect of the char at `index` (zero width, row height).
fn char_rect(galley: &Galley, galley_pos: Pos2, index: usize) -> Rect {
    galley.pos_from_cursor(CCursor::new(index)).translate(galley_pos.to_vec2())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_border_stands_out_with_the_keyboard_in_the_popup() {
        let style = egui::Style::default();
        let normal = popup_frame(&style, false).stroke;
        let focused = popup_frame(&style, true).stroke;
        assert!(focused.width > normal.width);
        assert_eq!(focused.color, style.visuals.selection.bg_fill);
        assert_eq!(normal, egui::Frame::popup(&style).stroke);
    }

    /// Draws `docs` in the popup for some frames, as a new popup. Returns its last rect.
    fn popup_rect(ctx: &egui::Context, docs: &str) -> Rect {
        let content = Content { problems: Vec::new(), docs: Some(docs.to_owned()) };
        let mut open = Open::new(0, Rect::from_min_size(Pos2::new(10.0, 10.0), egui::Vec2::ZERO), content, false);
        let mut rect = Rect::NOTHING;
        for _ in 0..4 {
            let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 900.0));
            let input = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| {
                rect = show(ui.ctx(), egui::Id::NULL.with("editor"), &mut open, 320.0);
                open.rect = Some(rect);
            });
            output.textures_delta.clear();
        }
        rect
    }

    #[test]
    fn a_popup_is_as_tall_as_its_text_whatever_the_last_one_was() {
        let ctx = egui::Context::default();
        let long = "a line\n".repeat(40);

        let first_long = popup_rect(&ctx, &long).height();
        assert!(first_long > 300.0, "up to the limit: {first_long}");

        let short = popup_rect(&ctx, "one line").height();
        assert!(short < 60.0, "as small as its text: {short}");

        let long_after_short = popup_rect(&ctx, &long).height();
        assert!((long_after_short - first_long).abs() < 1.0, "{long_after_short} is not {first_long}");
    }

    #[test]
    fn a_tall_editor_has_a_taller_popup() {
        assert_eq!(max_height(600.0), 320.0);
        assert_eq!(max_height(2000.0), 500.0);
    }
}
