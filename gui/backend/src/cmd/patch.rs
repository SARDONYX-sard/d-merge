use super::{bail, sender};
use mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use tauri::Window;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Mod info readers

/// # glob samples
/// - Steam VFS: `D:/Steam/steamapps/common/Skyrim Special Edition/Data`
/// - MO2: `D:/GAME/ModOrganizer Skyrim SE/mods/*`
#[tauri::command]
pub(crate) fn load_mods_info(glob: &str, ids: Vec<String>) -> Result<Vec<ModInfo>, String> {
    let pattern = format!("{glob}/Nemesis_Engine/mod/*/info.ini");
    let info = ModsInfo::get_all(&pattern).or_else(|err| bail!(err))?;

    let priority_map = ids
        .into_iter()
        .enumerate()
        .map(|(index, mod_name)| (mod_name, index + 1)) // +1: 1-based priority
        .collect();
    Ok(info.sort_to_vec_by_priority(priority_map))
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
pub(crate) fn patch(window: Window, output: &str, ids: Vec<String>) -> Result<(), String> {
    tracing::trace!(?output, ?ids);
    let _sender = sender::<Payload>(window, "d_merge://progress/patch");
    Ok(())
}
