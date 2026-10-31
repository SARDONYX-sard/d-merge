//! Find bar and "go to line" for [`Keymap::Normal`](crate::Keymap::Normal) (VS Code keys).
//! Vim has its own `/`, `n`, `N`, `*` and `:<line>` (see `keymap::vim::prompt`).
//!
//! ```text
//!                                   ┌──────────────────────────────────┐
//!                                   │ [animmotion        ] 2/5  ↑ ↓ ✖ │  <- top right of the editor
//!                                   └──────────────────────────────────┘
//! ```
//!
//! | Key                       | Action                                            |
//! | ------------------------- | ------------------------------------------------- |
//! | `Ctrl+F`                  | Find (pre-filled with the selection)              |
//! | `Enter` / `Shift+Enter`   | Next / previous match (in the find field)         |
//! | `F3` / `Shift+F3`         | Next / previous match (anywhere)                  |
//! | `Ctrl+F3`                 | Find the word under the cursor                    |
//! | `Ctrl+G`                  | Go to line                                        |
//! | `Esc`                     | Close                                             |
//!
//! Matching is the same as Vim's search: plain text, smartcase (see [`crate::search`]).

use std::ops::Range;

use egui::{Key, Modifiers};

use crate::{
    Keymap,
    search::{Matcher, find_from, is_word},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Find,
    GotoLine,
}

/// State of the bar (one per document).
#[derive(Debug, Default)]
pub(crate) struct FindBar {
    /// `Some` while the bar is shown.
    open: Option<Kind>,
    query: String,
    line: String,
    /// Focus the input field on the next draw.
    focus_field: bool,
    /// Char index the incremental search starts from (the cursor when the bar opened).
    origin: usize,
}

impl FindBar {
    /// The matches to highlight while the find bar is open.
    pub(crate) fn highlight(&self) -> Option<Matcher> {
        (self.open == Some(Kind::Find)).then(|| Matcher::new(&self.query)).flatten()
    }

    /// Global keys. Call before the `TextEdit`. `editor_focus`: the editor has the keyboard.
    ///
    /// Only for [`Keymap::Normal`]: Vim uses these keys itself (e.g., `Ctrl+F` = page down).
    pub(crate) fn handle_keys(
        &mut self,
        keymap: Keymap,
        ui: &egui::Ui,
        id: egui::Id,
        text: &str,
        editor_focus: bool,
    ) {
        if keymap != Keymap::Normal {
            self.open = None; // e.g., switched to Vim while the bar was open
            return;
        }
        if !editor_focus && self.open.is_none() {
            return;
        }
        // NOTE: `consume_key` ignores extra Shift/Alt: the Shift variant must be tried first.
        let (find, goto, word, prev, next, escape) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::F),
                i.consume_key(Modifiers::COMMAND, Key::G),
                i.consume_key(Modifiers::COMMAND, Key::F3),
                i.consume_key(Modifiers::SHIFT, Key::F3),
                i.consume_key(Modifiers::NONE, Key::F3),
                // egui drops the editor focus on `Esc` itself; just close the bar.
                editor_focus && i.key_pressed(Key::Escape),
            )
        });
        let selection = selection(ui.ctx(), id);

        if find {
            // Pre-fill with a one-line selection, like VS Code.
            if let Some(range) = selection.clone().filter(|r| !r.is_empty()) {
                let selected: String = text.chars().skip(range.start).take(range.len()).collect();
                if !selected.contains('\n') {
                    self.query = selected;
                }
            }
            self.open_as(Kind::Find, selection.as_ref());
        }
        if goto {
            self.line.clear();
            self.open_as(Kind::GotoLine, selection.as_ref());
        }
        if word && let Some(at) = selection.as_ref().map(|r| r.start) {
            let chars: Vec<char> = text.chars().collect();
            if let Some(word) = word_at(&chars, at) {
                self.query = word;
                self.open_as(Kind::Find, selection.as_ref());
                self.focus_field = false; // keep typing in the editor
                self.jump(ui.ctx(), id, text, true);
            }
        }
        if next || prev {
            self.jump(ui.ctx(), id, text, next);
        }
        if escape {
            self.open = None;
        }
    }

    /// Draws the bar at the top right of `editor_rect` (the visible part of the editor).
    pub(crate) fn ui(&mut self, ui: &egui::Ui, id: egui::Id, editor_rect: egui::Rect, text: &str) {
        let Some(kind) = self.open else { return };
        let ctx = ui.ctx();

        egui::Area::new(id.with("find_bar"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(editor_rect.right_top() + egui::vec2(-16.0, 4.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| match kind {
                        Kind::Find => self.find_ui(ui, id, text),
                        Kind::GotoLine => self.goto_ui(ui, id, text),
                    });
                });
            });
    }

    /// `[query] 2/5 ↑ ↓ ✖`
    fn find_ui(&mut self, ui: &mut egui::Ui, id: egui::Id, text: &str) {
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .hint_text("Find")
                .desired_width(200.0)
                .id(id.with("find_query")),
        );
        self.take_focus(&field);
        if field.changed() {
            self.select_from_origin(ui.ctx(), id, text);
        }

        let chars: Vec<char> = text.chars().collect();
        let matcher = Matcher::new(&self.query);
        let total = matcher.as_ref().map_or(0, |m| m.count(&chars));
        let current = selection(ui.ctx(), id)
            .and_then(|range| matcher.as_ref()?.index_of(&chars, range.start));
        let counter = match (total, current) {
            (0, _) if self.query.is_empty() => String::new(),
            (0, _) => "No results".to_string(),
            (total, Some(current)) => format!("{current}/{total}"),
            (total, None) => format!("?/{total}"),
        };
        ui.label(egui::RichText::new(counter).weak());

        let (enter, shift) = ui.input(|i| (i.key_pressed(Key::Enter), i.modifiers.shift));
        // A single-line `TextEdit` gives up the focus on `Enter`: take it back.
        let submitted = field.lost_focus() && enter;
        if submitted {
            self.jump(ui.ctx(), id, text, !shift);
            self.focus_field = true;
        }
        if ui.small_button("↑").on_hover_text("Previous match (Shift+Enter, Shift+F3)").clicked()
        {
            self.jump(ui.ctx(), id, text, false);
        }
        if ui.small_button("↓").on_hover_text("Next match (Enter, F3)").clicked() {
            self.jump(ui.ctx(), id, text, true);
        }
        self.close_ui(ui, id, &field, submitted);
    }

    /// `Go to line (1-120): [42] ✖`
    fn goto_ui(&mut self, ui: &mut egui::Ui, id: egui::Id, text: &str) {
        let lines = text.split('\n').count();
        ui.label(format!("Go to line (1-{lines}):"));
        let field = ui.add(
            egui::TextEdit::singleline(&mut self.line).desired_width(60.0).id(id.with("goto_line")),
        );
        self.take_focus(&field);

        let submitted = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if submitted {
            if let Ok(line) = self.line.trim().parse::<usize>() {
                let line = line.clamp(1, lines) - 1;
                let index = crate::text_pos::LinePos { line, column: 0 }.to_char_index(text);
                crate::set_cursor(ui.ctx(), id, index); // also focuses the editor
            }
            self.open = None;
            return;
        }
        self.close_ui(ui, id, &field, submitted);
    }

    /// `✖` button and `Esc` in the field: close and give the keyboard back to the editor.
    fn close_ui(
        &mut self,
        ui: &mut egui::Ui,
        id: egui::Id,
        field: &egui::Response,
        submitted: bool,
    ) {
        let escape = field.lost_focus() && !submitted && ui.input(|i| i.key_pressed(Key::Escape));
        if ui.small_button("✖").on_hover_text("Close (Esc)").clicked() || escape {
            self.open = None;
            ui.memory_mut(|m| m.request_focus(id));
        }
    }

    fn open_as(&mut self, kind: Kind, selection: Option<&Range<usize>>) {
        self.open = Some(kind);
        self.focus_field = true;
        self.origin = selection.map_or(0, |r| r.start);
    }

    fn take_focus(&mut self, field: &egui::Response) {
        if std::mem::take(&mut self.focus_field) {
            field.request_focus();
        }
    }

    /// While typing: select the first match at or after where the search started.
    fn select_from_origin(&self, ctx: &egui::Context, id: egui::Id, text: &str) {
        let Some(matcher) = Matcher::new(&self.query) else { return };
        let chars: Vec<char> = text.chars().collect();
        let at = if matcher.matches_at(&chars, self.origin) {
            Some(self.origin)
        } else {
            find_from(&matcher, &chars, self.origin, true)
        };
        if let Some(at) = at {
            select(ctx, id, at..at + self.query.chars().count());
        }
    }

    /// Selects the next (`forward`) or previous match from the cursor, wrapping around.
    fn jump(&self, ctx: &egui::Context, id: egui::Id, text: &str, forward: bool) {
        let Some(matcher) = Matcher::new(&self.query) else { return };
        let chars: Vec<char> = text.chars().collect();
        let from = selection(ctx, id).map_or(0, |r| r.start);
        if let Some(at) = find_from(&matcher, &chars, from, forward) {
            select(ctx, id, at..at + self.query.chars().count());
        }
    }
}

/// The selection of the editor `id` as a sorted char range.
fn selection(ctx: &egui::Context, id: egui::Id) -> Option<Range<usize>> {
    let range = egui::TextEdit::load_state(ctx, id)?.cursor.char_range()?;
    let (a, b) = (range.primary.index.0, range.secondary.index.0);
    Some(a.min(b)..a.max(b))
}

/// Selects `range` in the editor and scrolls it into view, **without** taking the focus
/// (the user keeps typing in the find field).
fn select(ctx: &egui::Context, id: egui::Id, range: Range<usize>) {
    use egui::text::{CCursor, CCursorRange};
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(range.start),
        CCursor::new(range.end),
    )));
    state.store(ctx, id);
    ctx.data_mut(|data| data.insert_temp(crate::scroll_request_id(id), true));
}

/// The word (`[A-Za-z0-9_]+`, Unicode letters too) at or right before `at`.
fn word_at(chars: &[char], at: usize) -> Option<String> {
    let at = if chars.get(at).is_some_and(|&c| is_word(c)) { at } else { at.checked_sub(1)? };
    if !is_word(*chars.get(at)?) {
        return None;
    }
    let start = chars[..at].iter().rposition(|&c| !is_word(c)).map_or(0, |i| i + 1);
    let end = chars[at..].iter().position(|&c| !is_word(c)).map_or(chars.len(), |i| at + i);
    Some(chars[start..end].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_under_cursor() {
        let chars: Vec<char> = "0.5 animmotion 1".chars().collect();
        assert_eq!(word_at(&chars, 6).as_deref(), Some("animmotion"));
        assert_eq!(word_at(&chars, 14).as_deref(), Some("animmotion")); // right after the word
        assert_eq!(word_at(&chars, 3), Some("5".to_string()));
    }
}
