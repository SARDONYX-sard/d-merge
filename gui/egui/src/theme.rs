use d_merge_gui_shared::settings::ui::theme::{Rgba, Theme, ThemePreset};

// NOTE: I made a few changes to the code for fine-tuning the theme because it was hard to read when viewed in the GUI.
fn new_shadcn_dark() -> egui_shadcn::ShadcnTheme {
    let mut theme = egui_shadcn::theme::shadcn_theme_dark::dark();
    theme.foreground = egui::Color32::from_rgb(200, 200, 200);
    theme
}

fn new_shadcn_light() -> egui_shadcn::ShadcnTheme {
    let mut theme = egui_shadcn::theme::shadcn_theme_light::light();
    theme.muted = egui::Color32::from_rgb(190, 190, 190).gamma_multiply(0.8); // button hover
    theme
}

/// A theme setup routine that runs only once before the GUI app is launched
pub(crate) fn setup_theme(
    ctx: &egui::Context,
    settings: &d_merge_gui_shared::settings::Settings,
) -> crate::ui::theme::ThemeManager {
    let theme_manager = crate::ui::theme::ThemeManager::new(
        &settings.ui.custom_theme.themes_dir,
        settings.ui.custom_theme.selected_theme.as_deref(),
    );
    set_theme(ctx, settings.ui.theme, theme_manager.editing.as_ref());

    theme_manager
}

pub(crate) fn set_theme(ctx: &egui::Context, theme: Theme, theme_preset: Option<&ThemePreset>) {
    let (visuals, shadcn_theme) = match theme {
        Theme::System => match dark_light::detect() {
            Ok(dark_light::Mode::Light) => (egui::Visuals::light(), new_shadcn_light()),
            _ => (egui::Visuals::dark(), new_shadcn_dark()),
        },
        Theme::Dark => (egui::Visuals::dark(), new_shadcn_dark()),
        Theme::Light => (egui::Visuals::light(), new_shadcn_light()),
        Theme::Custom => {
            if let Some(theme_preset) = theme_preset {
                crate::ui::theme::apply(theme_preset, ctx);
            }
            return;
        }
    };

    ctx.set_style_of(ctx.theme(), egui::Style { visuals, ..Default::default() });
    egui_shadcn::ShadcnThemeExt::set_shadcn_theme(ctx, shadcn_theme);
}

/// Applies theme-dependent styling to a top/bottom panel.
///
/// The panel background is determined by `bg_color`.
pub(crate) fn themed_top_bottom_panel(
    panel: egui::Panel,
    theme: Theme,
    bg_color: Option<&Rgba>,
) -> egui::Panel {
    panel.frame(frame_from_theme(theme, bg_color))
}

/// Applies theme-dependent styling to a central panel.
///
/// The panel background is determined by `bg_color`.
pub(crate) fn themed_central_panel(
    panel: egui::CentralPanel,
    theme: Theme,
    bg_color: Option<&Rgba>,
) -> egui::CentralPanel {
    panel.frame(frame_from_theme(theme, bg_color))
}

fn frame_from_theme(theme: Theme, bg_color: Option<&Rgba>) -> egui::Frame {
    let effective_theme = match theme {
        Theme::System => match dark_light::detect() {
            Ok(dark_light::Mode::Light) => Theme::Light,
            _ => Theme::Dark,
        },
        other => other,
    };

    let fill = match effective_theme {
        Theme::Light => egui::Color32::WHITE,
        Theme::Dark => egui::Color32::from_rgba_unmultiplied(30, 30, 30, 255),
        Theme::Custom => bg_color.map(|c| c.to_egui_color32()).unwrap_or_default(),
        Theme::System => unreachable!(),
    };

    egui::Frame::NONE.fill(fill)
}

/// Conversion between configuration colors and egui colors.
pub(crate) trait EguiColorExt {
    /// Converts to an [`egui::Color32`].
    fn to_egui_color32(self) -> egui::Color32;

    /// Converts from an [`egui::Color32`].
    fn from_egui_color32(color: egui::Color32) -> Self;
}

impl EguiColorExt for Rgba {
    #[inline]
    fn to_egui_color32(self) -> egui::Color32 {
        egui::Color32::from_rgba_premultiplied(self.r, self.g, self.b, self.a)
    }

    #[inline]
    fn from_egui_color32(color: egui::Color32) -> Self {
        Self::new(color.r(), color.g(), color.b(), color.a())
    }
}
