mod mod_info_loader;

use crate::cmd::{bail, time};
use crate::error::NotFoundResourceDirSnafu;
pub(crate) use mod_info_loader::{
    __cmd__get_skyrim_data_dir, __cmd__load_mods_info, get_skyrim_data_dir, load_mods_info,
};
use nemesis_merge::{
    behavior_gen, Config, DebugOptions, HackOptions, OutPutTarget, PatchMaps, Status,
};
use once_cell::sync::Lazy;
use snafu::ResultExt as _;
use std::path::{Path, PathBuf};
use tauri::{async_runtime::JoinHandle, path::BaseDirectory, AppHandle, Emitter as _, Manager};
use tokio::sync::Mutex;

const PATCH_STATUS_EVENT_NAME: &str = "d_merge://progress/patch";
static PATCH_TASK: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

/// Emits a patch status event to the frontend.
fn emit_status(app: &AppHandle, payload: Status) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(PATCH_STATUS_EVENT_NAME, payload);
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuiPatchOptions {
    hack_options: Option<HackOptions>,
    debug: DebugOptions,
    output_target: OutPutTarget,
    auto_remove_meshes: bool,
    use_progress_reporter: bool,

    /// Skyrim data directories glob (required **only when using FNIS**).
    ///
    /// This must include all directories containing `animations/<namespace>`, otherwise FNIS
    /// entries will not be detected and the process will fail.
    skyrim_data_dir_glob: Option<String>,
}

// TODO: To prevent emit failures, use AppHandle instead of Window. (However, the validity of this has not been tested.)
#[tauri::command]
pub(crate) async fn patch(
    app: AppHandle,
    output: PathBuf,
    patches: PatchMaps,
    options: GuiPatchOptions,
) -> Result<(), String> {
    tracing::info!("Starting patch with options: {options:#?}");

    cancel_patch_inner().await?; // Abort previous task if exists

    if options.auto_remove_meshes {
        let meshes_path = output.join("meshes");
        let debug_path = output.join(".d_merge").join(".debug");
        tokio::join!(remove_if_exists(meshes_path), remove_if_exists(debug_path),);
    }

    let resource_dir = app
        .path()
        .resolve("assets/templates", BaseDirectory::Resource)
        .context(NotFoundResourceDirSnafu)
        .or_else(|err| bail!(err))?;

    let app_handle = app.clone();
    let handle = tauri::async_runtime::spawn(async move {
        let status_report = options.use_progress_reporter.then(|| {
            let cloned_app_handle = app.clone();
            Box::new(move |payload: Status| emit_status(&cloned_app_handle, payload))
                as Box<dyn Fn(Status) + Send + Sync>
        });

        let config = Config {
            output_dir: output,
            resource_dir,
            status_report,
            hack_options: options.hack_options,
            debug: options.debug,
            output_target: options.output_target,
            skyrim_data_dir_glob: options.skyrim_data_dir_glob,
        };

        match time!("[patch]", behavior_gen(patches, config).await) {
            Ok(()) => emit_status(&app_handle, Status::Done),
            Err(err) => emit_status(&app_handle, Status::Error(err)),
        }
    });

    PATCH_TASK.lock().await.replace(handle);
    Ok(())
}

/// Removes a directory if it exists, with debug logging.
async fn remove_if_exists<P>(path: P)
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.exists() {
        tracing::debug!("Starting removal of `{}`", path.display());
        match tokio::fs::remove_dir_all(path).await {
            Ok(_) => tracing::debug!("Successfully removed at `{}`", path.display()),
            Err(e) => tracing::error!("Failed to remove at `{}`: {e}", path.display()),
        }
    }
}

#[tauri::command]
pub async fn cancel_patch() -> Result<(), String> {
    cancel_patch_inner().await
}

async fn cancel_patch_inner() -> Result<(), String> {
    let handle = {
        let mut guard = PATCH_TASK.lock().await;
        guard.take()
    };

    if let Some(handle) = handle {
        handle.abort();
        if let Err(err) = handle.await {
            tracing::error!("patch task panicked: {err}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_option() {
        let gui_options = GuiPatchOptions {
            hack_options: Some(HackOptions::enable_all()),
            debug: DebugOptions::enable_all(),
            output_target: OutPutTarget::SkyrimSe,
            ..Default::default()
        };
        let json = serde_json::to_string_pretty(&gui_options).unwrap();
        println!("{json}");
    }
}
