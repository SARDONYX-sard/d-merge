//! Completion popup.
//!
//! # Lifecycle
//! ```text
//! typing a trigger char (Language::completion_triggers) or a word char, or Ctrl+Space
//!     -> refresh(): ask the language for candidates at the cursor; open if any
//! while open:
//!     ArrowUp/Down  -> move the selection
//!     Enter / Tab   -> accept: replace the typed word with the candidate
//!     Escape        -> close
//!     cursor leaves the line / no candidates -> close
//! ```
//!
//! Keys are consumed **before** the `TextEdit` runs, otherwise Enter would insert a newline.

use super::{
    language::{CompletionList, Language},
    text_pos::{LinePos, char_to_byte},
};

#[derive(Debug, Default)]
pub(super) struct CompletionPopup {
    /// `Some` while open.
    list: Option<CompletionList>,
    /// Line the list was computed for; moving to another line closes the popup.
    line: usize,
    /// Highlighted candidate.
    selected: usize,
}

/// Result of accepting a candidate.
pub(super) struct Accepted {
    pub(super) text: String,
    /// Char range to select afterwards (a placeholder, or an empty range after the insertion).
    pub(super) select: std::ops::Range<usize>,
}

impl CompletionPopup {
    pub(super) const fn is_open(&self) -> bool {
        self.list.is_some()
    }

    pub(super) fn close(&mut self) {
        self.list = None;
    }

    /// Consumes the navigation keys while open. Returns `true` when the user accepted.
    /// Call before showing the `TextEdit`.
    pub(super) fn handle_keys(&mut self, ui: &egui::Ui) -> bool {
        let Some(len) = self.list.as_ref().map(|list| list.items.len()) else { return false };
        let consume = |key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));

        if consume(egui::Key::Escape) {
            self.close();
            return false;
        }
        if consume(egui::Key::ArrowDown) {
            self.selected = (self.selected + 1) % len;
        }
        if consume(egui::Key::ArrowUp) {
            self.selected = (self.selected + len - 1) % len;
        }
        consume(egui::Key::Enter) || consume(egui::Key::Tab)
    }

    /// Recomputes candidates at `cursor`. Opens/closes the popup accordingly.
    pub(super) fn refresh(&mut self, language: &dyn Language, text: &str, cursor: LinePos) {
        let line = text.split('\n').nth(cursor.line).unwrap_or_default();
        let list = language.complete(line, cursor.column);

        if list.items.is_empty() {
            self.close();
        } else {
            self.selected = self.selected.min(list.items.len() - 1);
            self.line = cursor.line;
            self.list = Some(list);
        }
    }

    /// Decides whether the edit made this frame should (re)open the popup.
    ///
    /// e.g., typing `.` after `PIE` opens it; typing while it is open refreshes (filters) it.
    pub(super) fn on_text_changed(
        &mut self,
        ui: &egui::Ui,
        language: &dyn Language,
        text: &str,
        cursor: LinePos,
    ) {
        let triggers = language.completion_triggers();
        let typed_trigger = ui.input(|i| {
            i.events.iter().any(|event| match event {
                egui::Event::Text(s) => s
                    .chars()
                    .last()
                    .is_some_and(|c| triggers.contains(&c) || c.is_alphanumeric() || c == '_'),
                _ => false,
            })
        });

        if typed_trigger || self.is_open() {
            self.refresh(language, text, cursor);
        }
    }

    /// Closes the popup when the cursor moved to another line.
    pub(super) fn follow_cursor(&mut self, cursor: Option<LinePos>) {
        if self.is_open() && cursor.is_none_or(|c| c.line != self.line) {
            self.close();
        }
    }

    /// Applies the selected candidate to `text`.
    pub(super) fn accept(&mut self, text: &str) -> Option<Accepted> {
        let list = self.list.take()?;
        let item = list.items.get(self.selected)?;

        // Document char index of the replaced range.
        let line_start = LinePos { line: self.line, column: 0 }.to_char_index(text);
        let start = line_start + list.replace.start;
        let end = line_start + list.replace.end;

        let mut new_text = String::with_capacity(text.len() + item.insert.len());
        new_text.push_str(&text[..char_to_byte(text, start)]);
        new_text.push_str(&item.insert);
        new_text.push_str(&text[char_to_byte(text, end)..]);

        let after = start + item.insert.chars().count();
        let select = item.select.as_ref().map_or(after..after, |r| start + r.start..start + r.end);
        Some(Accepted { text: new_text, select })
    }

    /// Draws the list below `anchor` (the cursor rect in screen coordinates).
    ///
    /// Returns `true` when a candidate was clicked (the caller then calls [`Self::accept`]).
    pub(super) fn ui(&mut self, ctx: &egui::Context, id: egui::Id, anchor: egui::Rect) -> bool {
        let Some(list) = &self.list else { return false };
        let mut clicked = false;

        egui::Area::new(id.with("completion"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor.left_bottom())
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        // Candidates
                        ui.vertical(|ui| {
                            for (i, item) in list.items.iter().enumerate() {
                                let label = format!("{} {}", item.icon, item.label);
                                let response =
                                    ui.add(egui::Button::selectable(i == self.selected, label));
                                if response.hovered() {
                                    self.selected = i;
                                }
                                clicked |= response.clicked();
                            }
                        });

                        // Documentation of the selected candidate
                        if let Some(item) = list.items.get(self.selected)
                            && !item.documentation.is_empty()
                        {
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_max_width(360.0);
                                egui_widgets::markdown::markdown(ui, &item.documentation);
                            });
                        }
                    });
                });
            });

        clicked
    }
}
