//! Draws the find bar and the "go to line" bar. The state and the search are in [`crate::find`].

use egui::Key;

use crate::{
    find::{FindBar, Kind},
    search::index_of,
};

impl FindBar {
    /// Draws the bar at the top right of `editor_rect` (the visible part of the editor), and
    /// marks the current match (the editor shows no selection while the bar has the focus).
    pub(crate) fn ui(
        &mut self,
        ui: &egui::Ui,
        id: egui::Id,
        editor_rect: egui::Rect,
        text: &str,
        (galley, galley_pos): (&egui::Galley, egui::Pos2),
    ) {
        let Some(kind) = self.open else { return };
        let ctx = ui.ctx();
        if kind == Kind::Find
            && let Some(current) = self.current_match(ctx, id, text)
        {
            crate::ui::overlays::block_selection(ui, galley, galley_pos, &[current]);
        }

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

    /// `[query] .* 2/5 ↑ ↓ ✖`
    pub(crate) fn find_ui(&mut self, ui: &mut egui::Ui, id: egui::Id, text: &str) {
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

        // `.*`, as VS Code's: the query is a regex.
        let regex = egui::Button::selectable(self.regex, egui::RichText::new(".*").monospace()).small();
        if ui.add(regex).on_hover_text("Use regular expression").clicked() {
            self.regex = !self.regex;
            self.select_from_origin(ui.ctx(), id, text);
        }
        let matches = self.matcher().map(|matcher| matcher.find_all(text)).unwrap_or_default();
        let total = matches.len();
        let current = self.current_match(ui.ctx(), id, text).and_then(|range| index_of(&matches, range.start));
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
        if ui.small_button("↑").on_hover_text("Previous match (Shift+Enter, Shift+F3)").clicked() {
            self.jump(ui.ctx(), id, text, false);
        }
        if ui.small_button("↓").on_hover_text("Next match (Enter, F3)").clicked() {
            self.jump(ui.ctx(), id, text, true);
        }
        self.close_ui(ui, id, &field, submitted);
    }

    /// `Go to line (1-120): [42] ✖`
    pub(crate) fn goto_ui(&mut self, ui: &mut egui::Ui, id: egui::Id, text: &str) {
        let lines = text.split('\n').count();
        ui.label(format!("Go to line (1-{lines}):"));
        let field = ui.add(egui::TextEdit::singleline(&mut self.line).desired_width(60.0).id(id.with("goto_line")));
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
    pub(crate) fn close_ui(&mut self, ui: &mut egui::Ui, id: egui::Id, field: &egui::Response, submitted: bool) {
        let escape = field.lost_focus() && !submitted && ui.input(|i| i.key_pressed(Key::Escape));
        if ui.small_button("✖").on_hover_text("Close (Esc)").clicked() || escape {
            self.open = None;
            ui.memory_mut(|m| m.request_focus(id));
        }
    }

    pub(crate) fn take_focus(&mut self, field: &egui::Response) {
        if std::mem::take(&mut self.focus_field) {
            field.request_focus();
        }
    }
}
