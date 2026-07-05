// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod fonts;
pub(crate) mod theme;
mod ui;

use d_merge_gui_shared::{log::LOG_FILENAME, settings};

use crate::theme::set_theme;

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

    let options = create_native_options(&settings);
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            crate::fonts::setup_fonts(&cc.egui_ctx, &settings.ui.font);
            let theme_manager = crate::theme::setup_theme(&cc.egui_ctx, &settings);
            Ok(Box::new(self::app::App::new(settings, theme_manager)))
        }),
    )
}

/// Create native window options.
fn create_native_options(settings: &settings::Settings) -> eframe::NativeOptions {
    let (icon_rgba, [icon_width, icon_height]) = d_merge_gui_shared::d_merge_icon();

    eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            title: Some(APP_TITLE.to_string()),
            app_id: Some("D Merge".to_string()),
            position: Some(egui::Pos2::new(settings.ui.window.pos_x, settings.ui.window.pos_y)),
            // Avoid `NoGlutinConfigs` on Linux.
            transparent: Some({
                #[cfg(target_os = "linux")]
                {
                    false
                }
                #[cfg(not(target_os = "linux"))]
                {
                    true
                }
            }),
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
    }
}
