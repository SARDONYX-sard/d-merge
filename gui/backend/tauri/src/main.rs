// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cmd;
mod error;
mod log;

// NOTE: For some reason, other tasks freeze after executing async cmd, so I don't use #[tokio::main].
fn main() {
    #[allow(clippy::large_stack_frames)]
    if let Err(err) = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state_init())
        .on_window_event(prevent_close_window)
        .invoke_handler(tauri::generate_handler![
            crate::cmd::conversion::convert,
            crate::cmd::conversion::load_dir_node,
            crate::cmd::fs::write_file,
            crate::cmd::log::change_log_level,
            crate::cmd::patch::cancel_patch,
            crate::cmd::patch::get_skyrim_data_dir,
            crate::cmd::patch::load_mods_info,
            crate::cmd::patch::patch,
            crate::cmd::set_vfs_mode,
            crate::cmd::updater::fetch_versions,
            crate::cmd::updater::update_to_version,
        ])
        .setup(|app| Ok(crate::log::init(app)?))
        .run(tauri::generate_context!())
    {
        tracing::error!("Error: {err}");
        std::process::exit(1);
    }
}

/// -ref: [Avoid auto show(To avoid white flash screen) issue](https://github.com/tauri-apps/plugins-workspace/issues/344)
///
/// And it's necessary to call `show()` in frontend to show the Window.
fn tauri_plugin_window_state_init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_window_state::StateFlags;
    tauri_plugin_window_state::Builder::default()
        .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
        .build()
}

/// This is there to wait until the front end saves the current status.
///
/// Since the window cannot be closed, it is necessary to call `getCurrentWindow().destroy()` in js to close the Window.
/// To prevent exit application by X button.
fn prevent_close_window<R: tauri::Runtime>(window: &tauri::Window<R>, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if cmd::IS_VFS_MODE.load(std::sync::atomic::Ordering::Acquire) {
            // The closing process is not done here, as the front end is responsible for saving and closing the settings.
            api.prevent_close();
            return;
        }

        if let Err(err) = window.close() {
            tracing::error!("{err}");
        }
    }
}
