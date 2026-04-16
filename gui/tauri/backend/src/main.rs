// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cmd;
mod error;
mod log;

// NOTE: For some reason, other tasks freeze after executing async cmd, so I don't use #[tokio::main].
fn main() {
    // tauri: v2.5 emit crash hack: https://github.com/tauri-apps/tauri/issues/10987#issuecomment-3687624898
    // It could be avoid async run freeze?
    #[expect(clippy::expect_used)]
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");
    tauri::async_runtime::set(runtime.handle().clone());

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
            crate::cmd::hkanno::load_hkanno,
            crate::cmd::hkanno::preview_hkanno,
            crate::cmd::hkanno::save_hkanno,
            crate::cmd::log::change_log_level,
            crate::cmd::log::log,
            crate::cmd::patch::mod_info_loader::get_skyrim_data_dir,
            crate::cmd::patch::mod_info_loader::load_mods_info,
            crate::cmd::patch::patch,
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
fn prevent_close_window<R: tauri::Runtime>(_: &tauri::Window<R>, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
    }
}
