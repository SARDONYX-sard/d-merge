use super::{bail, sender};
use crate::error::NotFoundResourceDirSnafu;
use mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use nemesis_merge::{behavior_gen, Options};
use snafu::ResultExt as _;
use std::path::PathBuf;
use tauri::{Manager, Window};

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Mod info readers

/// # glob samples
/// - Steam VFS: `D:/Steam/steamapps/common/Skyrim Special Edition/Data`
/// - MO2: `D:/GAME/ModOrganizer Skyrim SE/mods/*`
#[tauri::command]
pub(crate) fn load_mods_info(glob: &str) -> Result<Vec<ModInfo>, String> {
    let pattern = format!("{glob}/Nemesis_Engine/mod/*/info.ini");
    let info = ModsInfo::get_all(&pattern).or_else(|err| bail!(err))?;
    Ok(info)
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// # Progress report for progress bar
///
/// - First: number of files/dirs explored
/// - After: working index
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Payload {
    /// - First: number of files/dirs explored
    /// - After: working index
    index: usize,
}

#[tauri::command]
pub(crate) async fn patch(window: Window, output: &str, ids: Vec<PathBuf>) -> Result<(), String> {
    tracing::trace!(?output, ?ids);

    let resolver = window.app_handle().path();
    // Expected `<ResourceDir>/assets/templates/meshes/[..]`
    // - ref https://v2.tauri.app/develop/resources/
    let resource_dir = resolver
        .resource_dir()
        .context(NotFoundResourceDirSnafu)
        .or_else(|err| bail!(err))?
        .join("resource/assets/templates/");
    behavior_gen(
        ids,
        Options {
            output_dir: PathBuf::from(output),
            resource_dir,
        },
    )
    .await
    .or_else(|err| bail!(err))?;
    let _sender = sender::<Payload>(window, "d_merge://progress/patch");
    Ok(())
}
