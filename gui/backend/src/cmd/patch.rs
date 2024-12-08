use super::{bail, sender, time};
use crate::error::NotFoundResourceDirSnafu;
use mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use nemesis_merge::{behavior_gen, Config, Status};
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

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[tauri::command]
pub(crate) async fn patch(window: Window, output: &str, ids: Vec<PathBuf>) -> Result<(), String> {
    // Expected `<ResourceDir>/assets/templates/meshes/[..]`
    // - ref https://v2.tauri.app/develop/resources/
    let resource_dir = {
        let resolver = window.app_handle().path();
        resolver
            .resource_dir()
            .context(NotFoundResourceDirSnafu)
            .or_else(|err| bail!(err))?
            .join("assets/templates/")
    };

    let status_reporter = sender::<Status>(window, "d_merge://progress/patch");
    time! {
        "[patch]",
        behavior_gen(
            ids,
            Config {
                output_dir: PathBuf::from(output),
                resource_dir,
                status_report: Some(Box::new(status_reporter)),
            },
        ).await
    }
}
