// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets
//! shadcn-styled drop-ins for [`egui::TextEdit`] and [`egui::Label`].

use super::theme::{ShadcnTheme, interpolate_color, paint_focus_ring};

/// A framed single-line text input, like [`egui::TextEdit::singleline`].
///
/// The response is the inner [`egui::TextEdit`]'s one.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct TextEdit<'a> {
    text: &'a mut dyn egui::TextBuffer,
    hint_text: egui::WidgetText,
    desired_width: Option<f32>,
    id_salt: Option<egui::IdSalt>,
    password: bool,
    char_limit: usize,
    interactive: bool,
    height: f32,
}

/// Default of [`TextEdit::height`] and [`NumberInput::height`]: `h-8`.
const FIELD_HEIGHT: f32 = 32.0;

impl std::fmt::Debug for TextEdit<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEdit").field("text", &self.text.as_str()).finish_non_exhaustive()
    }
}

impl<'a> TextEdit<'a> {
    #[inline]
    pub fn singleline(text: &'a mut dyn egui::TextBuffer) -> Self {
        Self {
            text,
            hint_text: egui::WidgetText::default(),
            desired_width: None,
            id_salt: None,
            password: false,
            char_limit: usize::MAX,
            interactive: true,
            height: FIELD_HEIGHT,
        }
    }

    /// The field's height (not in egui), e.g. smaller in a dense list.
    #[inline]
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// A source for the unique [`egui::Id`], e.g. `.id_salt("second_text_edit_field")`.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::IdSalt::new(id_salt));
        self
    }

    /// If true, hide the letters from view and prevent copying from the field.
    #[inline]
    pub const fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Sets the limit for the amount of characters can be entered
    #[inline]
    pub const fn char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    /// Default is `true`. If set to `false` then you cannot interact with the text.
    #[inline]
    pub const fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Show a faint hint text when the text field is empty.
    #[inline]
    pub fn hint_text(mut self, hint_text: impl Into<egui::WidgetText>) -> Self {
        self.hint_text = hint_text.into();
        self
    }

    /// Defaults to the available width.
    #[inline]
    pub const fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }
}

impl egui::Widget for TextEdit<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());

        let width = self.desired_width.unwrap_or_else(|| ui.available_width());
        let (outer_rect, inner_rect) = field_frame(ui, &theme, egui::vec2(width, self.height));
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new().max_rect(inner_rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
        );

        let mut text_edit = egui::TextEdit::singleline(self.text)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(4, 2)))
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
            field_focus(ui, &theme, outer_rect);
        }
        response
    }
}

/// The frame of a field (its background, its border lighter while pointed at): allocates it and
/// returns `(the frame, inside its padding)`.
fn field_frame(ui: &mut egui::Ui, theme: &ShadcnTheme, size: egui::Vec2) -> (egui::Rect, egui::Rect) {
    let h_padding: f32 = 10.0; // px-2.5
    let v_padding: f32 = if size.y < FIELD_HEIGHT { 1.0 } else { 4.0 }; // py-1
    let cr = theme.corner_radius();
    let base_bg = interpolate_color(theme.background, theme.muted, 0.4);

    let (outer_rect, outer_response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let outer_hovered = outer_response.hovered() || ui.rect_contains_pointer(outer_rect);
    let (bg, border_color) = if outer_hovered {
        (interpolate_color(base_bg, theme.accent, 0.35), theme.input)
    } else {
        (base_bg, theme.border)
    };
    ui.painter().rect_filled(outer_rect, cr, bg);
    let border = egui::Stroke::new(1.0_f32, border_color);
    ui.painter().rect_stroke(outer_rect, cr, border, egui::epaint::StrokeKind::Inside);

    let inner_rect = egui::Rect::from_min_max(
        egui::pos2(outer_rect.min.x + h_padding, outer_rect.min.y + v_padding),
        egui::pos2(outer_rect.max.x - h_padding, outer_rect.max.y - v_padding),
    );
    (outer_rect, inner_rect)
}

/// The ring of a field with the keyboard.
fn field_focus(ui: &egui::Ui, theme: &ShadcnTheme, outer_rect: egui::Rect) {
    let stroke = egui::Stroke::new(1.0_f32, theme.ring);
    ui.painter().rect_stroke(outer_rect, theme.corner_radius(), stroke, egui::epaint::StrokeKind::Inside);
    paint_focus_ring(ui.painter(), outer_rect, theme.radius, theme.ring);
}

/// A row as high as a field from the start, its contents in its middle: a label put in before a
/// field is beside it, not above (in the middle of the lower row `ui.horizontal` starts with).
///
/// e.g. `Output format: [HKX SE (64-bit) ▼]`
pub fn field_row<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let row = egui::vec2(ui.available_width(), FIELD_HEIGHT);
    ui.allocate_ui_with_layout(row, egui::Layout::left_to_right(egui::Align::Center), add_contents)
}

/// A label as high as a field, its text in its middle: e.g. in a grid's cell beside a field.
pub fn field_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    field_row(ui, |ui| ui.label(text)).inner
}

/// A framed number, dragged or typed, like [`egui::DragValue`] in the frame of [`TextEdit`].
///
/// The response is the inner [`egui::DragValue`]'s one.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct NumberInput<'a> {
    inner: egui::DragValue<'a>,
    desired_width: Option<f32>,
    height: f32,
}

impl std::fmt::Debug for NumberInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumberInput").field("height", &self.height).finish_non_exhaustive()
    }
}

impl<'a> NumberInput<'a> {
    #[inline]
    pub fn new<Num: egui::emath::Numeric>(value: &'a mut Num) -> Self {
        Self { inner: egui::DragValue::new(value), desired_width: None, height: FIELD_HEIGHT }
    }

    /// How much the value changes per point dragged.
    #[inline]
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.inner = self.inner.speed(speed);
        self
    }

    /// The values it can take.
    #[inline]
    pub fn range<Num: egui::emath::Numeric>(mut self, range: std::ops::RangeInclusive<Num>) -> Self {
        self.inner = self.inner.range(range);
        self
    }

    /// At most this many decimals are shown and kept.
    #[inline]
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.inner = self.inner.max_decimals(max_decimals);
        self
    }

    /// Defaults to the available width.
    #[inline]
    pub const fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// The field's height (not in egui), e.g. smaller in a dense list.
    #[inline]
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl egui::Widget for NumberInput<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());
        let width = self.desired_width.unwrap_or_else(|| ui.available_width());
        let (outer_rect, inner_rect) = field_frame(ui, &theme, egui::vec2(width, self.height));
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new().max_rect(inner_rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
        );

        // The frame is the field's: egui's own is not drawn.
        let widgets = &mut child_ui.visuals_mut().widgets;
        for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
            state.bg_fill = egui::Color32::TRANSPARENT;
            state.weak_bg_fill = egui::Color32::TRANSPARENT;
            state.bg_stroke = egui::Stroke::NONE;
            state.fg_stroke.color = theme.foreground;
        }
        let response = child_ui.add_sized(inner_rect.size(), self.inner);
        if response.has_focus() || response.dragged() {
            field_focus(ui, &theme, outer_rect);
        }
        response
    }
}

/// Static text: `text-sm font-medium`, like [`egui::Label`].
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Label {
    text: egui::WidgetText,
}

impl Label {
    #[inline]
    pub fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl egui::Widget for Label {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let color = ShadcnTheme::get(ui.ctx()).foreground;
        let galley = ui.painter().layout_no_wrap(self.text.text().to_owned(), egui::FontId::proportional(14.0), color);

        let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().galley(rect.min, galley, color);
        }

        response
    }
}
