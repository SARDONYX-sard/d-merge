mod mod_info_loader;

use std::path::PathBuf;

pub(crate) use mod_info_loader::{
    __cmd__get_skyrim_data_dir, __cmd__load_mods_info, get_skyrim_data_dir, load_mods_info,
};
use nemesis_merge::{
    behavior_gen, Config, DebugOptions, HackOptions, OutPutTarget, PatchMaps, Status,
};
use once_cell::sync::Lazy;
use snafu::ResultExt as _;
use tauri::{path::BaseDirectory, AppHandle, Emitter as _, Manager};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    cmd::{bail, time},
    error::NotFoundResourceDirSnafu,
};

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
    /// Delete the meshes in the output destination each time the patch is run.
    auto_remove_meshes: bool,
    use_progress_reporter: bool,

    /// Skyrim data directories glob (required **only when using FNIS**).
    ///
    /// This must include all directories containing `animations/<namespace>`, otherwise FNIS
    /// entries will not be detected and the process will fail.
    skyrim_data_dir_glob: Option<String>,

    /// If true, generates a FNIS.esp(dummy ESP) file with the correct version and author information.
    pub generate_fnis_esp: Option<bool>,
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

    {
        use nemesis_merge::cache_remover;

        let is_dangerous_remove = options
            .skyrim_data_dir_glob
            .as_deref()
            .is_some_and(|d| cache_remover::is_dangerous_remove(&output, d));
        if is_dangerous_remove {
            tracing::warn!("0/6: The `auto remove meshes` option is checked, but the output directory is the Skyrim data directory.\nSince deleting meshes in that location risks destroying mods, the process was skipped.");
        } else {
            if options.auto_remove_meshes {
                cache_remover::remove_meshes_dir_all(&output);
            }
        }
    }

    let resource_dir = app
        .path()
        .resolve("assets/templates", BaseDirectory::Resource)
        .context(NotFoundResourceDirSnafu)
        .or_else(|err| bail!(err))?;

    let app_handle = app.clone();
    let handle = tokio::spawn(async move {
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
            generate_fnis_esp: options.generate_fnis_esp.unwrap_or(false),
        };

        match time!("[patch]", behavior_gen(patches, config).await) {
            Ok(()) => emit_status(&app_handle, Status::Done),
            Err(err) => emit_status(&app_handle, Status::Error(err)),
        }
    });

    PATCH_TASK.lock().await.replace(handle);
    Ok(())
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
