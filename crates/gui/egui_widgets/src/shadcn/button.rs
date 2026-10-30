// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/widgets/button
//! shadcn-styled drop-in for [`egui::Button`].

use super::{
    icon::{Icon, paint_icon},
    select::{ItemStyle, paint_item},
    theme::{ShadcnTheme, interpolate_color, paint_focus_ring, with_alpha},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonVariant {
    /// Border only.
    Outline,
    /// Primary background with primary foreground text.
    Primary,
    /// Tinted destructive background.
    Destructive,
    /// Toggle: accent background when selected.
    Selectable { selected: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonSize {
    Small,
    Normal,
    Large,
}

impl ButtonSize {
    /// Returns (height, horizontal_padding, font_size) in logical pixels.
    const fn metrics(self) -> (f32, f32, f32) {
        match self {
            Self::Small => (24.0, 8.0, 12.0),
            Self::Normal => (32.0, 10.0, 14.0),
            Self::Large => (36.0, 10.0, 14.0),
        }
    }
}

/// Clickable button, like [`egui::Button`]. Outlined by default.
///
/// Extra (non-egui) builder methods: [`Self::icon`], [`Self::large`], [`Self::primary`],
/// [`Self::destructive`], [`Self::full_width`].
#[derive(Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Button {
    text: egui::WidgetText,
    icon: Option<Icon>,
    variant: ButtonVariant,
    size: ButtonSize,
    min_size: egui::Vec2,
    full_width: bool,
}

impl Button {
    pub fn new(text: impl Into<egui::WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            variant: ButtonVariant::Outline,
            size: ButtonSize::Normal,
            min_size: egui::Vec2::ZERO,
            full_width: false,
        }
    }

    /// Creates a toggle-style button, like [`egui::Button::selectable`].
    ///
    /// Inside a [`super::ComboBox`] popup it renders as a menu item instead.
    pub fn selectable(selected: bool, text: impl Into<egui::WidgetText>) -> Self {
        Self { variant: ButtonVariant::Selectable { selected }, ..Self::new(text) }
    }

    /// Make this a small button.
    pub const fn small(mut self) -> Self {
        self.size = ButtonSize::Small;
        self
    }

    /// Set the minimum size of the button.
    pub const fn min_size(mut self, min_size: egui::Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    // ── Extensions (not in egui) ────────────────────────────────

    /// Shows an icon left of the text (or alone if the text is empty).
    pub const fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Make this a large button.
    pub const fn large(mut self) -> Self {
        self.size = ButtonSize::Large;
        self
    }

    /// Filled with the primary color.
    pub const fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    /// Tinted with the destructive color.
    pub const fn destructive(mut self) -> Self {
        self.variant = ButtonVariant::Destructive;
        self
    }

    /// Stretches to fill the allocated width, adopting the UI's own sizing.
    pub const fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
}

impl egui::Widget for Button {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        // Inside a `ComboBox` popup, selectable buttons are menu items.
        if let ButtonVariant::Selectable { selected } = self.variant
            && let Some(style) = ItemStyle::find(ui)
        {
            return paint_item(ui, style, selected, &self.text);
        }

        let theme = ShadcnTheme::get(ui.ctx());

        let (mut height, mut h_padding, mut font_size) = self.size.metrics();
        // When full_width, adopt the UI's own sizing so the button matches
        // native egui menu items (same height, font, padding).
        if self.full_width {
            height = ui.spacing().interact_size.y;
            if let Some(font_id) = ui.style().text_styles.get(&egui::TextStyle::Button) {
                font_size = font_id.size;
            }
            h_padding = ui.spacing().button_padding.x;
        }

        let text = self.text.text();
        let has_text = !text.is_empty();
        let text_galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(font_size),
            egui::Color32::PLACEHOLDER,
        );

        let icon_size = height * 0.5;
        let icon_gap = 6.0;

        let content_width = match (self.icon.is_some(), has_text) {
            (true, false) => height,
            (true, true) => h_padding + icon_size + icon_gap + text_galley.size().x + h_padding,
            (false, _) => text_galley.size().x + h_padding * 2.0,
        };
        let desired = egui::vec2(content_width, height).max(self.min_size);

        // full_width: use allocate_at_least so the button stretches to fill
        // the menu/list width without inflating the menu's own desired size.
        let (rect, response) = if self.full_width {
            ui.allocate_at_least(desired, egui::Sense::click())
        } else {
            ui.allocate_exact_size(desired, egui::Sense::click())
        };

        if ui.is_rect_visible(rect) {
            let (bg, fg, border) = resolve_colors(
                &theme,
                self.variant,
                response.hovered(),
                response.is_pointer_button_down_on(),
                !ui.is_enabled(),
            );

            let painter = ui.painter();
            let cr = theme.corner_radius();

            painter.rect_filled(rect, cr, bg);
            if let Some(border) = border {
                painter.rect_stroke(
                    rect,
                    cr,
                    egui::Stroke::new(1.0_f32, border),
                    egui::epaint::StrokeKind::Inside,
                );
            }

            let icon_square = egui::vec2(icon_size, icon_size);
            match self.icon {
                Some(icon) if !has_text => {
                    let icon_rect = egui::Rect::from_center_size(rect.center(), icon_square);
                    paint_icon(painter, icon_rect, icon, fg);
                }
                Some(icon) => {
                    let total_w = icon_size + icon_gap + text_galley.size().x;
                    let start_x = rect.center().x - total_w / 2.0;
                    let icon_rect = egui::Rect::from_min_size(
                        egui::pos2(start_x, rect.center().y - icon_size / 2.0),
                        icon_square,
                    );
                    paint_icon(painter, icon_rect, icon, fg);

                    let text_pos = egui::pos2(
                        start_x + icon_size + icon_gap,
                        rect.center().y - text_galley.size().y / 2.0,
                    );
                    painter.galley(text_pos, text_galley, fg);
                }
                None => {
                    let text_pos = rect.center() - text_galley.size() / 2.0;
                    painter.galley(text_pos, text_galley, fg);
                }
            }

            if response.has_focus() {
                paint_focus_ring(painter, rect, theme.radius, theme.ring);
            }
        }

        if matches!(self.variant, ButtonVariant::Selectable { .. }) && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }
}

/// Returns (bg, fg, border) for the given variant and interaction state.
fn resolve_colors(
    theme: &ShadcnTheme,
    variant: ButtonVariant,
    hovered: bool,
    active: bool,
    disabled: bool,
) -> (egui::Color32, egui::Color32, Option<egui::Color32>) {
    let (mut bg, mut fg, border) = match variant {
        ButtonVariant::Outline => (theme.background, theme.foreground, Some(theme.border)),
        ButtonVariant::Primary => (theme.primary, theme.primary_foreground, None),
        // Nova: bg-destructive/10 text-destructive (tinted, not solid)
        ButtonVariant::Destructive => (with_alpha(theme.destructive, 26), theme.destructive, None),
        ButtonVariant::Selectable { selected } => {
            let mut bg = if selected {
                theme.accent
            } else if hovered {
                theme.muted
            } else {
                egui::Color32::TRANSPARENT
            };
            if active {
                bg = interpolate_color(bg, theme.accent, 0.65);
            }
            return (bg, theme.foreground, None);
        }
    };

    if disabled {
        if bg.a() > 0 {
            bg = with_alpha(bg, 128);
        }
        fg = theme.muted_foreground;
    } else if active {
        bg = with_alpha(bg, 204); // 80%
    } else if hovered {
        if variant == ButtonVariant::Outline {
            bg = theme.muted;
            fg = theme.foreground;
        } else {
            bg = with_alpha(bg, 204);
        }
    }

    (bg, fg, border)
}
