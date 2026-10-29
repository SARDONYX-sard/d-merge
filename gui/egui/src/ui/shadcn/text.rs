// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets
//! shadcn-styled drop-ins for [`egui::TextEdit`] and [`egui::Label`].

use super::theme::{ShadcnTheme, interpolate_color, paint_focus_ring};

/// A framed single-line text input, like [`egui::TextEdit::singleline`].
///
/// The response is the inner [`egui::TextEdit`]'s one.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub(crate) struct TextEdit<'a> {
    text: &'a mut dyn egui::TextBuffer,
    hint_text: egui::WidgetText,
    desired_width: Option<f32>,
    id_salt: Option<egui::Id>,
    password: bool,
    char_limit: usize,
    interactive: bool,
}

impl std::fmt::Debug for TextEdit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEdit").field("text", &self.text.as_str()).finish_non_exhaustive()
    }
}

impl<'a> TextEdit<'a> {
    pub(crate) fn singleline(text: &'a mut dyn egui::TextBuffer) -> Self {
        Self {
            text,
            hint_text: egui::WidgetText::default(),
            desired_width: None,
            id_salt: None,
            password: false,
            char_limit: usize::MAX,
            interactive: true,
        }
    }

    /// A source for the unique [`egui::Id`], e.g. `.id_salt("second_text_edit_field")`.
    pub(crate) fn id_salt(mut self, id_salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(egui::Id::new(id_salt));
        self
    }

    /// If true, hide the letters from view and prevent copying from the field.
    pub(crate) const fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Sets the limit for the amount of characters can be entered
    pub(crate) const fn char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    /// Default is `true`. If set to `false` then you cannot interact with the text.
    pub(crate) const fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Show a faint hint text when the text field is empty.
    pub(crate) fn hint_text(mut self, hint_text: impl Into<egui::WidgetText>) -> Self {
        self.hint_text = hint_text.into();
        self
    }

    /// Defaults to the available width.
    pub(crate) const fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }
}

impl egui::Widget for TextEdit<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());

        let height: f32 = 32.0; // h-8
        let h_padding: f32 = 10.0; // px-2.5
        let v_padding: f32 = 4.0; // py-1
        let width = self.desired_width.unwrap_or_else(|| ui.available_width());

        let cr = theme.corner_radius();
        let base_bg = interpolate_color(theme.background, theme.muted, 0.4);

        let (outer_rect, outer_response) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let outer_hovered = outer_response.hovered() || ui.rect_contains_pointer(outer_rect);
        let (bg, border_color) = if outer_hovered {
            (interpolate_color(base_bg, theme.accent, 0.35), theme.input)
        } else {
            (base_bg, theme.border)
        };

        ui.painter().rect_filled(outer_rect, cr, bg);
        ui.painter().rect_stroke(
            outer_rect,
            cr,
            egui::Stroke::new(1.0_f32, border_color),
            egui::epaint::StrokeKind::Inside,
        );

        // Place the TextEdit inside, with padding
        let inner_rect = egui::Rect::from_min_max(
            egui::pos2(outer_rect.min.x + h_padding, outer_rect.min.y + v_padding),
            egui::pos2(outer_rect.max.x - h_padding, outer_rect.max.y - v_padding),
        );
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(inner_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );

        let mut text_edit = egui::TextEdit::singleline(self.text)
            .frame(false)
            .hint_text(self.hint_text)
            .text_color(theme.foreground)
            .desired_width(inner_rect.width())
            .password(self.password)
            .char_limit(self.char_limit)
            .interactive(self.interactive);
        if let Some(id_salt) = self.id_salt {
            text_edit = text_edit.id_salt(id_salt);
        }
        let response = child_ui.add(text_edit);

        if response.has_focus() {
            ui.painter().rect_stroke(
                outer_rect,
                cr,
                egui::Stroke::new(1.0_f32, theme.ring),
                egui::epaint::StrokeKind::Inside,
            );
            paint_focus_ring(ui.painter(), outer_rect, theme.radius, theme.ring);
        }

        response
    }
}

/// Static text: `text-sm font-medium`, like [`egui::Label`].
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub(crate) struct Label {
    text: egui::WidgetText,
}

impl Label {
    pub(crate) fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl egui::Widget for Label {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let color = ShadcnTheme::get(ui.ctx()).foreground;
        let galley = ui.painter().layout_no_wrap(
            self.text.text().to_owned(),
            egui::FontId::proportional(14.0),
            color,
        );

        let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().galley(rect.min, galley, color);
        }

        response
    }
}
