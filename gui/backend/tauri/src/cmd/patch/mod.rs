mod mod_info_loader;

pub(crate) use mod_info_loader::{
    __cmd__get_skyrim_data_dir, __cmd__load_mods_info, get_skyrim_data_dir, load_mods_info,
};
use tauri::path::BaseDirectory;

use crate::cmd::{bail, time};
use crate::error::NotFoundResourceDirSnafu;
use nemesis_merge::{
    behavior_gen, Config, DebugOptions, HackOptions, OutPutTarget, PatchMaps, Status,
};
use once_cell::sync::Lazy;
use snafu::ResultExt as _;
use std::path::{Path, PathBuf};
use tauri::{async_runtime::JoinHandle, Manager, Window};
use tokio::sync::Mutex;

static PATCH_TASK: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

fn sender<S>(window: Window, event: &'static str) -> Box<dyn Fn(S) + Send + Sync>
where
    S: serde::Serialize + Clone + Send + Sync + 'static,
{
    Box::new(move |payload: S| {
        if let Err(err) = tauri::Emitter::emit(&window, event, payload) {
            tracing::error!("{}", err);
        };
    })
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
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
    pub skyrim_data_dir_glob: Option<String>,
}

#[tauri::command]
pub(crate) async fn patch(
    window: Window,
    output: PathBuf,
    patches: PatchMaps,
    options: GuiPatchOptions,
) -> Result<(), String> {
    cancel_patch_inner().await?; // Abort previous task if exists

    if options.auto_remove_meshes {
        let meshes_path = output.join("meshes");
        let debug_path = output.join(".d_merge").join(".debug");
        tokio::join!(remove_if_exists(meshes_path), remove_if_exists(debug_path),);
    }

    let handle = tauri::async_runtime::spawn({
        let resource_dir = {
            let app = window.app_handle();
            app.path()
                .resolve("assets/templates", BaseDirectory::Resource)
                .context(NotFoundResourceDirSnafu)
                .or_else(|err| bail!(err))?
        };

        async move {
            let status_report = match options.use_progress_reporter {
                true => Some(sender::<Status>(window, "d_merge://progress/patch")),
                false => None,
            };

            let config = Config {
                output_dir: output,
                resource_dir,
                status_report,
                hack_options: options.hack_options,
                debug: options.debug,
                output_target: options.output_target,
                skyrim_data_dir_glob: options.skyrim_data_dir_glob,
            };

            let _ = time!("[patch]", behavior_gen(patches, config).await);
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
