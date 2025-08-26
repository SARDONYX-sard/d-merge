#![allow(clippy::unwrap_used, clippy::expect_used)]
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod dnd;
mod fonts;
mod log;
mod mod_item;
mod settings;
mod ui;

use app::ModManagerApp;

/// Application entry point.
///
/// # Errors
/// Returns an error if the native GUI cannot be started.
fn main() -> Result<(), eframe::Error> {
    let _ = tracing_rotation::init(log::LOG_DIR, "d_merge.log");
    tracing_rotation::change_level("debug").unwrap();

    let settings = settings::AppSettings::load();
    let (icon_rgba, icon_size) = ico_to_rgba(include_bytes!("../../backend/tauri/icons/icon.ico"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder {
            title: Some("D Merge".to_string()),
            position: Some(egui::Pos2::new(
                settings.window_pos_x,
                settings.window_pos_y,
            )),
            maximized: Some(settings.window_maximized),
            inner_size: Some(egui::vec2(settings.window_width, settings.window_height)),
            resizable: Some(true),
            transparent: Some(true),
            icon: Some(std::sync::Arc::new(egui::IconData {
                rgba: icon_rgba,
                width: icon_size[0],
                height: icon_size[1],
            })),
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "D Merge",
        options,
        Box::new(|cc| {
            fonts::setup_custom_fonts(&cc.egui_ctx, settings.font_path.as_ref());
            Ok(Box::new(ModManagerApp::from(settings)))
        }),
    )
}

fn ico_to_rgba(bytes: &[u8]) -> (Vec<u8>, [u32; 2]) {
    let cursor = std::io::Cursor::new(bytes);
    let ico = ico::IconDir::read(cursor).unwrap();
    let entry = ico.entries().first().unwrap();
    let image = entry.decode().unwrap();
    let width = image.width();
    let height = image.height();
    (image.rgba_data().to_vec(), [width, height])
}
