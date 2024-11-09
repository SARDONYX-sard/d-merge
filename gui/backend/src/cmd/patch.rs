use super::{bail, sender};
use mod_info::{GetModsInfo as _, ModInfo, ModsInfo};
use std::collections::HashMap;
use tauri::Window;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Mod info readers

#[tauri::command]
pub(crate) fn load_mods_info() -> Result<Vec<ModInfo>, String> {
    let dir = "../../dummy/mods";
    let pattern = format!("{dir}/*/info.ini");
    let info = ModsInfo::get_all(&pattern).or_else(|err| bail!(err))?;
    Ok(info.sort_to_vec_by_priority(HashMap::new()))
}

#[tauri::command]
pub(crate) fn load_activate_mods() -> Result<Vec<String>, String> {
    let mods = "
aaaaa
bcbi"
        .lines();
    Ok(mods.into_iter().map(|m| m.to_string()).collect())
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
pub(crate) fn patch(window: Window, ids: Vec<String>) -> Result<(), String> {
    tracing::trace!("{:?}", ids);
    let _sender = sender::<Payload>(window, "d_merge://progress/patch");
    Ok(())
}
