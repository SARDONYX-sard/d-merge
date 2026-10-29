// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/theme
//! shadcn color tokens, stored in `egui::Context` temp data.

use egui::Color32;

/// All color tokens from the shadcn/ui design system, pre-converted to sRGB.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ShadcnTheme {
    pub(crate) background: Color32,
    pub(crate) foreground: Color32,
    pub(crate) card: Color32,
    pub(crate) card_foreground: Color32,
    pub(crate) popover: Color32,
    pub(crate) popover_foreground: Color32,
    pub(crate) primary: Color32,
    pub(crate) primary_foreground: Color32,
    pub(crate) secondary: Color32,
    pub(crate) secondary_foreground: Color32,
    pub(crate) muted: Color32,
    pub(crate) muted_foreground: Color32,
    pub(crate) accent: Color32,
    pub(crate) accent_foreground: Color32,
    pub(crate) destructive: Color32,
    pub(crate) destructive_foreground: Color32,
    pub(crate) border: Color32,
    pub(crate) input: Color32,
    pub(crate) ring: Color32,
    pub(crate) radius: f32,
}

impl ShadcnTheme {
    /// Dark theme matching shadcn/ui's Nova dark mode.
    pub(crate) fn dark() -> Self {
        Self {
            background: Color32::from_rgb(10, 10, 10),
            foreground: Color32::from_rgb(250, 250, 250),
            card: Color32::from_rgb(23, 23, 23),
            card_foreground: Color32::from_rgb(250, 250, 250),
            popover: Color32::from_rgb(23, 23, 23),
            popover_foreground: Color32::from_rgb(250, 250, 250),
            primary: Color32::from_rgb(229, 229, 229),
            primary_foreground: Color32::from_rgb(23, 23, 23),
            secondary: Color32::from_rgb(38, 38, 38),
            secondary_foreground: Color32::from_rgb(250, 250, 250),
            muted: Color32::from_rgb(38, 38, 38),
            muted_foreground: Color32::from_rgb(161, 161, 161),
            accent: Color32::from_rgb(64, 64, 64),
            accent_foreground: Color32::from_rgb(250, 250, 250),
            destructive: Color32::from_rgb(255, 99, 105),
            destructive_foreground: Color32::from_rgb(255, 255, 255),
            border: Color32::from_rgba_unmultiplied(255, 255, 255, 26),
            input: Color32::from_rgba_unmultiplied(255, 255, 255, 38),
            ring: Color32::from_rgb(115, 115, 115),
            radius: 10.0,
        }
    }

    /// Light theme matching shadcn/ui's Nova light mode.
    pub(crate) const fn light() -> Self {
        Self {
            background: Color32::from_rgb(255, 255, 255),
            foreground: Color32::from_rgb(10, 10, 10),
            card: Color32::from_rgb(255, 255, 255),
            card_foreground: Color32::from_rgb(10, 10, 10),
            popover: Color32::from_rgb(255, 255, 255),
            popover_foreground: Color32::from_rgb(10, 10, 10),
            primary: Color32::from_rgb(23, 23, 23),
            primary_foreground: Color32::from_rgb(250, 250, 250),
            secondary: Color32::from_rgb(245, 245, 245),
            secondary_foreground: Color32::from_rgb(23, 23, 23),
            muted: Color32::from_rgb(245, 245, 245),
            muted_foreground: Color32::from_rgb(115, 115, 115),
            accent: Color32::from_rgb(245, 245, 245),
            accent_foreground: Color32::from_rgb(23, 23, 23),
            destructive: Color32::from_rgb(229, 72, 77),
            destructive_foreground: Color32::from_rgb(255, 255, 255),
            border: Color32::from_rgb(229, 229, 229),
            input: Color32::from_rgb(229, 229, 229),
            ring: Color32::from_rgb(161, 161, 161),
            radius: 10.0,
        }
    }

    /// Returns the current theme (light by default).
    pub(crate) fn get(ctx: &egui::Context) -> Self {
        ctx.data(|d| d.get_temp::<Self>(egui::Id::NULL)).unwrap_or_else(Self::light)
    }

    /// Sets the theme for all subsequent frames.
    pub(crate) fn set(self, ctx: &egui::Context) {
        // Also set egui visuals so built-in popups (e.g. context_menu) match
        ctx.global_style_mut(|style| {
            style.visuals.window_fill = self.popover;
            style.visuals.window_stroke = egui::Stroke::new(1.0_f32, self.border);
            style.visuals.window_shadow = popup_shadow();
        });
        ctx.data_mut(|d| d.insert_temp(egui::Id::NULL, self));
    }

    /// Corner radius from `radius`.
    pub(crate) const fn corner_radius(&self) -> egui::CornerRadius {
        egui::CornerRadius::same(self.radius.round() as u8)
    }
}

/// Shadow shared by popups.
pub(crate) const fn popup_shadow() -> egui::Shadow {
    egui::Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::from_black_alpha(8) }
}

/// Linearly interpolates between two colors by factor `t` (0.0 = a, 1.0 = b).
pub(crate) fn interpolate_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (f32::from(x) * (1.0 - t) + f32::from(y) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
        lerp(a.a(), b.a()),
    )
}

/// Returns `c` with its alpha replaced by `a`.
pub(crate) fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Paints a 3px focus ring around `rect`.
pub(crate) fn paint_focus_ring(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: f32,
    ring_color: Color32,
) {
    painter.rect_stroke(
        rect.expand(2.0),
        egui::CornerRadius::same((corner_radius + 2.0).round() as u8),
        egui::Stroke::new(3.0_f32, with_alpha(ring_color, 128)),
        egui::epaint::StrokeKind::Outside,
    );
}
