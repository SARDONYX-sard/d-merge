// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod fonts;
mod ui;

use d_merge_gui_shared::{
    log::LOG_FILENAME,
    settings::{self, ui::Theme},
};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub(crate) const APP_TITLE: &str = concat!("D Merge v", env!("CARGO_PKG_VERSION"));

/// Application entry point.
///
/// # Errors
/// Returns an error if the native GUI cannot be started.
fn main() -> Result<(), eframe::Error> {
    let (settings, err) = match settings::Settings::load() {
        Ok(s) => (s, None),
        Err(e) => (settings::Settings::default(), Some(e)),
    };

    let _ = tracing_rotation::global::init_with_level(
        settings.log.dir_path.as_str(),
        LOG_FILENAME,
        5,
        settings.log.level,
    );

    std::panic::set_hook(Box::new(|info| {
        tracing::error!(?info);
    }));

    if let Some(err) = err {
        tracing::error!("[Settings loader] {err}\nFallback to default");
    } else {
        tracing::info!("[Settings loader] Loaded settings.json");
    };

    let (icon_rgba, [icon_width, icon_height]) = d_merge_gui_shared::d_merge_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            title: Some(APP_TITLE.to_string()),
            app_id: Some("D Merge".to_string()),
            position: Some(egui::Pos2::new(settings.ui.window.pos_x, settings.ui.window.pos_y)),
            transparent: Some(true),
            maximized: Some(settings.ui.window.maximized),
            inner_size: Some(egui::vec2(settings.ui.window.width, settings.ui.window.height)),
            resizable: Some(true),
            icon: Some(std::sync::Arc::new(egui::IconData {
                rgba: icon_rgba,
                width: icon_width,
                height: icon_height,
            })),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            if let Err(err) = crate::fonts::setup_fonts(&cc.egui_ctx, &settings.ui.font) {
                match err {
                    crate::fonts::FontError::Warn(msg) => tracing::warn!(msg),
                    crate::fonts::FontError::Error(msg) => tracing::error!(msg),
                }
            }
            set_theme(&cc.egui_ctx, settings.ui.theme, settings.ui.transparent);

            Ok(Box::new(self::app::App::from_settings(settings)))
        }),
    )
}

pub(crate) fn set_theme(ctx: &egui::Context, theme: Theme, transparent: bool) {
    ctx.set_theme(match theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::Light => egui::ThemePreference::Light,
    });

    // NOTE: I made a few changes to the code for fine-tuning the theme because it was hard to read when viewed in the GUI.
    let theme = match theme {
        Theme::System | Theme::Dark => {
            let mut theme = egui_shadcn::theme::shadcn_theme_dark::dark();
            theme.foreground = egui::Color32::from_rgb(200, 200, 200);
            if transparent {
                theme.background = egui::Color32::from_rgb(46, 46, 46).gamma_multiply(0.8); // button bg
                theme.muted = egui::Color32::from_rgb(20, 20, 20).gamma_multiply(0.8); // button hover
            }
            theme
        }
        Theme::Light => {
            let mut theme = egui_shadcn::theme::shadcn_theme_light::light();
            if !transparent {
                theme.muted = egui::Color32::from_rgb(190, 190, 190).gamma_multiply(0.8); // button hover
            }
            theme
        }
    };

    egui_shadcn::ShadcnThemeExt::set_shadcn_theme(ctx, theme);
}
