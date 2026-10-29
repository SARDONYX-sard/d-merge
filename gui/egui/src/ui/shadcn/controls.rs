// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets
//! shadcn-styled drop-ins for [`egui::Checkbox`], [`egui::RadioButton`] and [`egui::Slider`],
//! plus a toggle group.

use super::theme::{ShadcnTheme, paint_focus_ring, with_alpha};

/// Boolean on/off control with text label, like [`egui::Checkbox`].
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub(crate) struct Checkbox<'a> {
    checked: &'a mut bool,
    text: egui::WidgetText,
}

impl<'a> Checkbox<'a> {
    pub(crate) fn new(checked: &'a mut bool, text: impl Into<egui::WidgetText>) -> Self {
        Self { checked, text: text.into() }
    }

    pub(crate) fn without_text(checked: &'a mut bool) -> Self {
        Self::new(checked, egui::WidgetText::default())
    }
}

impl egui::Widget for Checkbox<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self { checked, text } = self;
        let theme = ShadcnTheme::get(ui.ctx());

        let box_size = 16.0;
        let corner_radius = 4.0;
        let spacing = 8.0;

        // Use Button text style to match menu items
        let label_font = ui
            .style()
            .text_styles
            .get(&egui::TextStyle::Button)
            .cloned()
            .unwrap_or_else(|| egui::FontId::proportional(14.0));
        let galley =
            ui.painter().layout_no_wrap(text.text().to_owned(), label_font, theme.foreground);

        let row_height = ui.spacing().interact_size.y.max(box_size);
        let desired = egui::vec2(box_size + galley.size().x + spacing, row_height);
        let (rect, mut response) = ui.allocate_exact_size(desired, egui::Sense::click());

        if response.clicked() {
            *checked = !*checked;
            response.mark_changed();
        }

        let anim_t = ui.ctx().animate_bool_responsive(response.id, *checked);

        if ui.is_rect_visible(rect) {
            let (mut box_bg, mut box_border, check_color) = if *checked {
                (theme.primary, theme.primary, theme.primary_foreground)
            } else if response.hovered() {
                (egui::Color32::TRANSPARENT, theme.ring, egui::Color32::TRANSPARENT)
            } else {
                (egui::Color32::TRANSPARENT, theme.border, egui::Color32::TRANSPARENT)
            };
            if response.is_pointer_button_down_on() {
                box_border = theme.ring;
                if !*checked {
                    box_bg = theme.accent;
                }
            }

            let painter = ui.painter();
            let box_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.center().y - box_size / 2.0),
                egui::vec2(box_size, box_size),
            );

            let cr = egui::CornerRadius::same(corner_radius as u8);
            painter.rect_filled(box_rect, cr, box_bg);
            painter.rect_stroke(
                box_rect,
                cr,
                egui::Stroke::new(1.0_f32, box_border),
                egui::epaint::StrokeKind::Inside,
            );

            // Checkmark, scaled by anim_t
            if anim_t >= 0.01 {
                let c = box_rect.center();
                let s = box_rect.width() * 0.3 * anim_t;
                let points = vec![
                    egui::pos2(c.x - s, c.y),
                    egui::pos2(c.x - s * 0.25, c.y + s * 0.7),
                    egui::pos2(c.x + s, c.y - s * 0.5),
                ];
                painter.add(egui::Shape::line(points, egui::Stroke::new(2.0_f32, check_color)));
            }

            let text_pos =
                egui::pos2(box_rect.max.x + spacing, rect.center().y - galley.size().y / 2.0);
            painter.galley(text_pos, galley, theme.foreground);

            if response.has_focus() {
                paint_focus_ring(painter, box_rect, corner_radius, theme.ring);
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }
}

/// One out of several alternatives, like [`egui::RadioButton`].
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub(crate) struct RadioButton {
    checked: bool,
    text: egui::WidgetText,
}

impl RadioButton {
    pub(crate) fn new(checked: bool, text: impl Into<egui::WidgetText>) -> Self {
        Self { checked, text: text.into() }
    }
}

impl egui::Widget for RadioButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());

        let outer_radius = 8.0;
        let inner_radius = 4.0;
        let spacing = 8.0;

        let galley = ui.painter().layout_no_wrap(
            self.text.text().to_owned(),
            egui::FontId::proportional(14.0),
            theme.foreground,
        );

        let desired = egui::vec2(outer_radius * 2.0 + galley.size().x + spacing, 20.0);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

        // A click selects it; paint the selected state right away.
        let checked = self.checked || response.clicked();
        let anim_t = ui.ctx().animate_bool_responsive(response.id, checked);

        if ui.is_rect_visible(rect) {
            let circle_border =
                if response.is_pointer_button_down_on() || (!checked && response.hovered()) {
                    theme.ring
                } else if checked {
                    theme.primary
                } else {
                    theme.input // Nova: border-input instead of border
                };

            let painter = ui.painter();
            let center = egui::pos2(rect.min.x + outer_radius, rect.center().y);

            painter.circle_stroke(center, outer_radius, egui::Stroke::new(1.0_f32, circle_border));

            // Inner dot (animated)
            if checked && anim_t > 0.01 {
                painter.circle_filled(center, inner_radius * anim_t, theme.primary);
            }

            let text_pos = egui::pos2(
                rect.min.x + outer_radius * 2.0 + spacing,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, theme.foreground);

            if response.has_focus() {
                let ring_rect =
                    egui::Rect::from_center_size(center, egui::Vec2::splat(outer_radius * 2.0));
                paint_focus_ring(painter, ring_rect, outer_radius, theme.ring);
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }
}

/// A set of exclusive toggles: `inline-flex gap-0.5 rounded-lg bg-muted p-0.5`.
///
/// Not in egui; the API follows [`egui::ComboBox::show_index`].
#[derive(Debug, Default)]
#[must_use = "You should call .show_index()"]
pub(crate) struct ToggleGroup;

impl ToggleGroup {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Show a list of items with the given selected index.
    ///
    /// The response is marked as changed when the selection changes.
    #[allow(clippy::unused_self, reason = "builder-style like `egui::ComboBox`")]
    pub(crate) fn show_index<Text: Into<egui::WidgetText>>(
        self,
        ui: &mut egui::Ui,
        selected: &mut usize,
        len: usize,
        get: impl Fn(usize) -> Text,
    ) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());

        let mut changed = false;
        let mut response = egui::Frame::NONE
            .fill(theme.muted)
            .inner_margin(egui::Margin::same(2))
            .corner_radius(theme.corner_radius())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for i in 0..len {
                        if toggle_group_item(ui, &theme, i == *selected, get(i)).clicked()
                            && *selected != i
                        {
                            *selected = i;
                            changed = true;
                        }
                    }
                });
            })
            .response;

        if changed {
            response.mark_changed();
        }
        response
    }
}

fn toggle_group_item(
    ui: &mut egui::Ui,
    theme: &ShadcnTheme,
    selected: bool,
    text: impl Into<egui::WidgetText>,
) -> egui::Response {
    let font_size: f32 = 13.0;
    let h_pad: f32 = 10.0;
    let height: f32 = 28.0;

    let galley = ui.painter().layout_no_wrap(
        text.into().text().to_owned(),
        egui::FontId::proportional(font_size),
        theme.foreground,
    );

    let desired = egui::vec2(galley.size().x + h_pad * 2.0, height);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let corner = egui::CornerRadius::same((theme.radius.round() as u8).saturating_sub(1));

        let bg = if selected {
            theme.background
        } else if response.hovered() {
            with_alpha(theme.background, 128)
        } else {
            egui::Color32::TRANSPARENT
        };
        painter.rect_filled(rect, corner, bg);

        if selected {
            painter.rect_stroke(
                rect,
                corner,
                egui::Stroke::new(1.0_f32, with_alpha(theme.foreground, 13)),
                egui::epaint::StrokeKind::Inside,
            );
        }

        painter.galley(rect.center() - galley.size() / 2.0, galley, theme.foreground);
    }

    response
}

type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

/// Control a number by dragging a handle, like [`egui::Slider`].
///
/// Shows the current value to the right. Integers are rounded.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub(crate) struct Slider<'a> {
    get_set_value: GetSetValue<'a>,
    range: std::ops::RangeInclusive<f64>,
    integral: bool,
    step: Option<f64>,
    show_value: bool,
    decimals: Option<usize>,
    prefix: String,
    suffix: String,
    text: egui::WidgetText,
    width: Option<f32>,
}

impl std::fmt::Debug for Slider<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slider").field("range", &self.range).finish_non_exhaustive()
    }
}

impl<'a> Slider<'a> {
    pub(crate) fn new<Num: egui::emath::Numeric>(
        value: &'a mut Num,
        range: std::ops::RangeInclusive<Num>,
    ) -> Self {
        let get_set_value = move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        };
        Self {
            get_set_value: Box::new(get_set_value),
            range: range.start().to_f64()..=range.end().to_f64(),
            integral: Num::INTEGRAL,
            step: None,
            show_value: true,
            decimals: None,
            prefix: String::new(),
            suffix: String::new(),
            text: egui::WidgetText::default(),
            width: None,
        }
    }

    /// Show a text field showing the current value? Default: `true`.
    pub(crate) const fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    /// Show a prefix before the number, e.g. "x: "
    pub(crate) fn prefix(mut self, prefix: impl ToString) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m")
    pub(crate) fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    /// Show a text next to the slider (e.g. explaining what the slider controls).
    pub(crate) fn text(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the minimal change of the value. `0.0` means no step.
    pub(crate) fn step_by(mut self, step: f64) -> Self {
        self.step = (step != 0.0).then_some(step);
        self
    }

    /// Set the number of decimals to display. Default: 0 for integers, 2 otherwise.
    pub(crate) const fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.decimals = Some(num_decimals);
        self
    }

    // ── Extensions (not in egui) ────────────────────────────────

    /// Track width. Default: `min(available_width, 200)` (egui uses `spacing().slider_width`).
    pub(crate) const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}

impl egui::Widget for Slider<'_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let theme = ShadcnTheme::get(ui.ctx());

        ui.horizontal(|ui| {
            let track_height: f32 = 4.0;
            let handle_radius = 6.0;
            let total_height = handle_radius * 2.0 + 4.0;
            let width = self.width.unwrap_or_else(|| ui.available_width().min(200.0));

            let (rect, mut response) = ui.allocate_exact_size(
                egui::vec2(width, total_height),
                egui::Sense::click_and_drag(),
            );

            let (start, end) = (*self.range.start(), *self.range.end());
            let span = end - start;
            let usable_min = rect.min.x + handle_radius;
            let usable_max = rect.max.x - handle_radius;

            let mut value = (self.get_set_value)(None);
            if (response.dragged() || response.clicked())
                && let Some(pos) = response.interact_pointer_pos()
            {
                let t = ((pos.x - usable_min) / (usable_max - usable_min)).clamp(0.0, 1.0);
                let mut new_value = start + f64::from(t) * span;
                if let Some(step) = self.step {
                    new_value = (new_value / step).round() * step;
                }
                if self.integral {
                    new_value = new_value.round();
                }
                let new_value = (self.get_set_value)(Some(new_value.clamp(start, end)));

                #[allow(clippy::float_cmp)] // Only used as a `changed` flag.
                if new_value != value {
                    value = new_value;
                    response.mark_changed();
                }
            }

            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let track_y = rect.center().y;

                let t = if span > 0.0 { ((value - start) / span) as f32 } else { 0.0 };
                let handle_x = usable_min + (usable_max - usable_min) * t;

                let track_cr = egui::CornerRadius::same((track_height / 2.0).round() as u8);
                let track_rect = |max_x: f32| {
                    egui::Rect::from_min_max(
                        egui::pos2(usable_min, track_y - track_height / 2.0),
                        egui::pos2(max_x, track_y + track_height / 2.0),
                    )
                };
                painter.rect_filled(track_rect(usable_max), track_cr, theme.muted);
                painter.rect_filled(track_rect(handle_x), track_cr, theme.primary);

                // Handle (Nova: border-ring on thumb)
                let handle_center = egui::pos2(handle_x, track_y);
                painter.circle_filled(handle_center, handle_radius, theme.background);
                painter.circle_stroke(
                    handle_center,
                    handle_radius,
                    egui::Stroke::new(2.0_f32, theme.ring),
                );

                if response.has_focus() {
                    let handle_rect = egui::Rect::from_center_size(
                        handle_center,
                        egui::Vec2::splat(handle_radius * 2.0),
                    );
                    paint_focus_ring(painter, handle_rect, handle_radius, theme.ring);
                }
            }

            if self.show_value {
                let decimals = self.decimals.unwrap_or(if self.integral { 0 } else { 2 });
                ui.label(format!("{}{value:.decimals$}{}", self.prefix, self.suffix));
            }
            if !self.text.is_empty() {
                ui.label(self.text);
            }

            response
        })
        .inner
    }
}
