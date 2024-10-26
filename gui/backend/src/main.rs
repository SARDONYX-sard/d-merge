// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cmd;
mod error;
mod log;

use tauri_plugin_window_state::StateFlags;

fn main() {
    #[allow(clippy::large_stack_frames)]
    if let Err(err) = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            // Avoid auto show(To avoid white flash screen): https://github.com/tauri-apps/plugins-workspace/issues/344
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            crate::cmd::change_log_level,
            crate::cmd::load_activate_mods,
            crate::cmd::load_mods_info,
            crate::cmd::patch,
            crate::cmd::convert::convert,
            crate::cmd::write_file,
        ])
        .setup(|app| Ok(crate::log::init(app)?))
        .run(tauri::generate_context!())
    {
        tracing::error!("Error: {err}");
        std::process::exit(1);
    }
}
