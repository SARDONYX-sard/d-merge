use super::{bail, sender, time};
use crate::error::{NotFoundResourceDirSnafu, NotFoundSkyrimDataDirSnafu};
use mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use nemesis_merge::{behavior_gen, Config, HackOptions, Status};
use snafu::ResultExt as _;
use std::path::PathBuf;
use tauri::{Manager, Window};

/// Mod info readers
///
/// # glob samples
/// - Steam VFS: `D:/Steam/steamapps/common/Skyrim Special Edition/Data`
/// - MO2: `D:/GAME/ModOrganizer Skyrim SE/mods/*`
#[tauri::command]
pub(crate) fn load_mods_info(glob: &str) -> Result<Vec<ModInfo>, String> {
    let pattern = format!("{glob}/Nemesis_Engine/mod/*/info.ini");
    let info = ModsInfo::get_all(&pattern).or_else(|err| bail!(err))?;
    Ok(info)
}

/// Get skyrim se/vr directory.
///
/// e.g. `D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data`
#[tauri::command]
pub(crate) fn get_skyrim_data_dir(is_se: bool) -> Result<PathBuf, String> {
    use super::get_skyrim_dir::Runtime;

    let runtime = if is_se { Runtime::Se } else { Runtime::Vr };
    match crate::cmd::get_skyrim_dir::get_skyrim_data_dir(runtime)
        .with_context(|_| NotFoundSkyrimDataDirSnafu)
    {
        Ok(path) => Ok(path),
        Err(err) => bail!(err),
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

use once_cell::sync::Lazy;
use std::sync::Mutex;
use tauri::async_runtime::JoinHandle;

static PATCH_TASK: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

/// - ids: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
#[tauri::command]
pub(crate) async fn patch(
    window: Window,
    output: PathBuf,
    ids: Vec<PathBuf>,
) -> Result<(), String> {
    // Abort previous task if exists
    cancel_patch_inner()?;

    // Spawn new task
    let handle = tauri::async_runtime::spawn({
        let resource_dir = {
            let resolver = window.app_handle().path();
            resolver
                .resource_dir()
                .context(NotFoundResourceDirSnafu)
                .or_else(|err| bail!(err))?
                .join("assets/templates/")
        };

        let status_reporter = sender::<Status>(window, "d_merge://progress/patch");

        async move {
            let _ = time! {
                "[patch]",
                behavior_gen(
                    ids,
                    Config {
                        output_dir: output,
                        resource_dir,
                        status_report: Some(Box::new(status_reporter)),
                        hack_options: Some(HackOptions::enable_all()), // TODO: Create GUI hack control popup
                    },
                ).await
            };
        }
    });

    match PATCH_TASK.lock() {
        Ok(mut guard) => {
            *guard = Some(handle);
        }
        Err(err) => {
            bail!(format!("Failed to acquire lock: {err}"));
        }
    }

    Ok(())
}

#[tauri::command]
pub fn cancel_patch() -> Result<(), String> {
    cancel_patch_inner()
}

fn cancel_patch_inner() -> Result<(), String> {
    match PATCH_TASK.lock() {
        Ok(mut guard) => {
            if let Some(handle) = guard.take() {
                handle.abort();
            }

            Ok(())
        }
        Err(err) => {
            bail!(format!("Failed to acquire lock: {err}"));
        }
    }
}
