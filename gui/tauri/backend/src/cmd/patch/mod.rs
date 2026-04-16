pub(crate) mod mod_info_loader;

use std::path::PathBuf;

use nemesis_merge::{
    Config, DebugOptions, HackOptions, OutPutTarget, PatchMaps, Status, behavior_gen,
};
use once_cell::sync::Lazy;
use snafu::ResultExt as _;
use tauri::{AppHandle, Emitter as _, Manager, async_runtime::JoinHandle, path::BaseDirectory};
use tokio::sync::Mutex;

use crate::{
    cmd::{bail, time},
    error::NotFoundResourceDirSnafu,
};

static PATCH_TASK: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

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
    app_handle: AppHandle,
    output: PathBuf,
    patches: PatchMaps,
    options: GuiPatchOptions,
) -> Result<(), String> {
    tracing::info!("Starting patch with options: {options:#?}");

    cancel_patch_inner().await?; // Abort previous task if exists
    remove_prev_output_if_no_dangerous(&options, &output);

    let resource_dir = app_handle
        .path()
        .resolve("assets/templates", BaseDirectory::Resource)
        .context(NotFoundResourceDirSnafu)
        .or_else(|err| bail!(err))?;

    let handle = tauri::async_runtime::spawn(async move {
        let status_report = options.use_progress_reporter.then(|| {
            // NOTE: This is necessary because the coercion does not happen automatically through `Option` returned by `then()`.
            let f: Box<dyn Fn(Status) + Send + Sync> = Box::new(move |payload: Status| {
                let _ = app_handle.emit("d_merge://progress/patch", payload);
            });
            f
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

        let _ = time!("[patch]", behavior_gen(patches, config).await);
    });

    PATCH_TASK.lock().await.replace(handle);
    Ok(())
}

fn remove_prev_output_if_no_dangerous(options: &GuiPatchOptions, output: &std::path::Path) {
    use nemesis_merge::cache_remover;

    let is_dangerous_remove = options
        .skyrim_data_dir_glob
        .as_deref()
        .is_some_and(|d| cache_remover::is_dangerous_remove(output, d));
    if is_dangerous_remove {
        tracing::warn!(
            "0/6: The `auto remove meshes` option is checked, but the output directory is the Skyrim data directory.\nSince deleting meshes in that location risks destroying mods, the process was skipped."
        );
    } else {
        if options.auto_remove_meshes {
            cache_remover::remove_meshes_dir_all(output);
        }
    }
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
