//! Serializable application theme definitions.

use d_merge_gui_shared::settings::ui::theme::{
    Rgba, ShadcnThemeConfig, StrokeConfig, VisualsConfig, WidgetVisualsConfig, WidgetsConfig,
};

use crate::theme::EguiColorExt;

pub(super) fn to_egui_visuals(visual: &VisualsConfig) -> egui::Visuals {
    let mut v = if visual.dark_mode { egui::Visuals::dark() } else { egui::Visuals::light() };

    v.override_text_color = visual.override_text_color.as_ref().map(|c| c.to_egui_color32());
    v.hyperlink_color = visual.hyperlink_color.to_egui_color32();
    v.faint_bg_color = visual.faint_bg_color.to_egui_color32();
    v.extreme_bg_color = visual.extreme_bg_color.to_egui_color32();
    v.code_bg_color = visual.code_bg_color.to_egui_color32();
    v.warn_fg_color = visual.warn_fg_color.to_egui_color32();
    v.error_fg_color = visual.error_fg_color.to_egui_color32();
    v.window_fill = visual.window_fill.to_egui_color32();
    v.panel_fill = visual.panel_fill.to_egui_color32();
    v.window_stroke = egui::Stroke {
        width: visual.window_stroke.width,
        color: visual.window_stroke.color.to_egui_color32(),
    };
    v.window_corner_radius = visual.window_corner_radius.into();
    v.menu_corner_radius = visual.menu_corner_radius.into();
    v.button_frame = visual.button_frame;
    v.striped = visual.striped;
    v.slider_trailing_fill = visual.slider_trailing_fill;
    // TODO: disabled_alpha is on egui::Style, not Visuals, so callers must also
    // set ctx.style_mut(|s| s.disabled_alpha = visual.disabled_alpha).

    let map_wvc = |wvc: &WidgetVisualsConfig| egui::style::WidgetVisuals {
        bg_fill: wvc.bg_fill.to_egui_color32(),
        weak_bg_fill: wvc.weak_bg_fill.to_egui_color32(),
        bg_stroke: egui::Stroke {
            width: wvc.bg_stroke.width,
            color: wvc.bg_stroke.color.to_egui_color32(),
        },
        fg_stroke: egui::Stroke {
            width: wvc.fg_stroke.width,
            color: wvc.fg_stroke.color.to_egui_color32(),
        },
        corner_radius: wvc.corner_radius.into(),
        expansion: 0.0,
    };

    v.widgets.noninteractive = map_wvc(&visual.widgets.noninteractive);
    v.widgets.inactive = map_wvc(&visual.widgets.inactive);
    v.widgets.hovered = map_wvc(&visual.widgets.hovered);
    v.widgets.active = map_wvc(&visual.widgets.active);
    v.widgets.open = map_wvc(&visual.widgets.open);

    v
}

pub(super) fn to_shadcn_theme(shadcn_theme: &ShadcnThemeConfig) -> egui_shadcn::ShadcnTheme {
    let ShadcnThemeConfig {
        background,
        foreground,
        card,
        card_foreground,
        popover,
        popover_foreground,
        primary,
        primary_foreground,
        secondary,
        secondary_foreground,
        muted,
        muted_foreground,
        accent,
        accent_foreground,
        destructive,
        destructive_foreground,
        border,
        input,
        ring,
        radius,
    } = shadcn_theme;

    egui_shadcn::ShadcnTheme {
        background: background.to_egui_color32(),
        foreground: foreground.to_egui_color32(),
        card: card.to_egui_color32(),
        card_foreground: card_foreground.to_egui_color32(),
        popover: popover.to_egui_color32(),
        popover_foreground: popover_foreground.to_egui_color32(),
        primary: primary.to_egui_color32(),
        primary_foreground: primary_foreground.to_egui_color32(),
        secondary: secondary.to_egui_color32(),
        secondary_foreground: secondary_foreground.to_egui_color32(),
        muted: muted.to_egui_color32(),
        muted_foreground: muted_foreground.to_egui_color32(),
        accent: accent.to_egui_color32(),
        accent_foreground: accent_foreground.to_egui_color32(),
        destructive: destructive.to_egui_color32(),
        destructive_foreground: destructive_foreground.to_egui_color32(),
        border: border.to_egui_color32(),
        input: input.to_egui_color32(),
        ring: ring.to_egui_color32(),
        radius: *radius,
    }
}

#[expect(unused)]
pub(super) fn from_egui_visuals(visuals: &egui::Visuals, disabled_alpha: f32) -> VisualsConfig {
    let map_wvc = |wvc: &egui::style::WidgetVisuals| WidgetVisualsConfig {
        bg_fill: Rgba::from_egui_color32(wvc.bg_fill),
        weak_bg_fill: Rgba::from_egui_color32(wvc.weak_bg_fill),
        bg_stroke: StrokeConfig {
            width: wvc.bg_stroke.width,
            color: Rgba::from_egui_color32(wvc.bg_stroke.color),
        },
        fg_stroke: StrokeConfig {
            width: wvc.fg_stroke.width,
            color: Rgba::from_egui_color32(wvc.fg_stroke.color),
        },
        corner_radius: wvc.corner_radius.sw,
    };

    VisualsConfig {
        dark_mode: visuals.dark_mode,
        override_text_color: visuals.override_text_color.map(Rgba::from_egui_color32),
        hyperlink_color: Rgba::from_egui_color32(visuals.hyperlink_color),
        faint_bg_color: Rgba::from_egui_color32(visuals.faint_bg_color),
        extreme_bg_color: Rgba::from_egui_color32(visuals.extreme_bg_color),
        code_bg_color: Rgba::from_egui_color32(visuals.code_bg_color),
        warn_fg_color: Rgba::from_egui_color32(visuals.warn_fg_color),
        error_fg_color: Rgba::from_egui_color32(visuals.error_fg_color),
        window_fill: Rgba::from_egui_color32(visuals.window_fill),
        panel_fill: Rgba::from_egui_color32(visuals.panel_fill),

        window_stroke: StrokeConfig {
            width: visuals.window_stroke.width,
            color: Rgba::from_egui_color32(visuals.window_stroke.color),
        },

        window_corner_radius: visuals.window_corner_radius.sw,
        menu_corner_radius: visuals.menu_corner_radius.sw,

        button_frame: visuals.button_frame,
        striped: visuals.striped,
        slider_trailing_fill: visuals.slider_trailing_fill,
        disabled_alpha,

        widgets: WidgetsConfig {
            noninteractive: map_wvc(&visuals.widgets.noninteractive),
            inactive: map_wvc(&visuals.widgets.inactive),
            hovered: map_wvc(&visuals.widgets.hovered),
            active: map_wvc(&visuals.widgets.active),
            open: map_wvc(&visuals.widgets.open),
        },
        text_edit_bg_color: None,
    }
}

#[expect(unused)]
pub(super) fn from_shadcn_theme(theme: &egui_shadcn::ShadcnTheme) -> ShadcnThemeConfig {
    ShadcnThemeConfig {
        background: Rgba::from_egui_color32(theme.background),
        foreground: Rgba::from_egui_color32(theme.foreground),
        card: Rgba::from_egui_color32(theme.card),
        card_foreground: Rgba::from_egui_color32(theme.card_foreground),
        popover: Rgba::from_egui_color32(theme.popover),
        popover_foreground: Rgba::from_egui_color32(theme.popover_foreground),
        primary: Rgba::from_egui_color32(theme.primary),
        primary_foreground: Rgba::from_egui_color32(theme.primary_foreground),
        secondary: Rgba::from_egui_color32(theme.secondary),
        secondary_foreground: Rgba::from_egui_color32(theme.secondary_foreground),
        muted: Rgba::from_egui_color32(theme.muted),
        muted_foreground: Rgba::from_egui_color32(theme.muted_foreground),
        accent: Rgba::from_egui_color32(theme.accent),
        accent_foreground: Rgba::from_egui_color32(theme.accent_foreground),
        destructive: Rgba::from_egui_color32(theme.destructive),
        destructive_foreground: Rgba::from_egui_color32(theme.destructive_foreground),
        border: Rgba::from_egui_color32(theme.border),
        input: Rgba::from_egui_color32(theme.input),
        ring: Rgba::from_egui_color32(theme.ring),
        radius: theme.radius,
    }
}
