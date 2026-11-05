//! Tracks the pointer and draws the hover popup. The state and its content are in [`crate::hover`].

use egui::{Galley, Pos2, Rect, text::CCursor};

use crate::hover::{Content, HOVER_DELAY, HoverCtx, HoverPopup, Open, POPUP_MARGIN, content};

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

        // Vim `K` on an open `K` popup: move the keyboard into it (like Neovim's hover window).
        if open.pinned && take_shift_k(ui) {
            open.focused = true;
            open.focus_first_link = true;
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
        self.open = content(index, ctx).map(|content| Open {
            index,
            anchor,
            content,
            rect: None,
            pinned: true,
            focused: false,
            focus_first_link: false,
        });
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
            open.rect = Some(show(ui.ctx(), ctx.id, open));
            open.focus_first_link = false;
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
                self.open = Some(Open {
                    index,
                    anchor,
                    content,
                    rect: None,
                    pinned: false,
                    focused: false,
                    focus_first_link: false,
                });
            }
            None => self.open = None,
        }
    }
}

/// Keys while the keyboard is in the popup. Sets `open.focused = false` to close it.
fn focused_keys(ui: &egui::Ui, open: &mut Open, editor_id: egui::Id) {
    use egui::{Key, Modifiers};

    let (close, next, prev, clicked_outside) = ui.input_mut(|i| {
        let close = i.consume_key(Modifiers::NONE, Key::Escape) || i.consume_key(Modifiers::NONE, Key::Q);
        let next = i.consume_key(Modifiers::NONE, Key::J) || i.consume_key(Modifiers::NONE, Key::ArrowDown);
        let prev = i.consume_key(Modifiers::NONE, Key::K) || i.consume_key(Modifiers::NONE, Key::ArrowUp);
        let click = i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten();
        let outside = click.is_some_and(|pos| !open.rect.is_some_and(|rect| rect.contains(pos)));
        (close, next, prev, outside)
    });

    if close || clicked_outside {
        open.focused = false;
        ui.memory_mut(|m| m.request_focus(editor_id));
    } else if next {
        ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Next));
    } else if prev {
        ui.memory_mut(|m| m.move_focus(egui::FocusDirection::Previous));
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
                    let links = egui_widgets::markdown::markdown(ui, docs);
                    if open.focus_first_link
                        && let Some(first) = links.first()
                    {
                        first.request_focus();
                    }
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
    let near_glyph =
        (pointer.x - glyph.left()).abs() <= 40.0 && glyph.expand2(egui::vec2(0.0, 2.0)).y_range().contains(pointer.y);
    near_glyph.then_some(cursor.index.0)
}

/// Screen rect of the char at `index` (zero width, row height).
fn char_rect(galley: &Galley, galley_pos: Pos2, index: usize) -> Rect {
    galley.pos_from_cursor(CCursor::new(index)).translate(galley_pos.to_vec2())
}
