use crate::cmd::bail;
use crate::error::NotFoundSkyrimDataDirSnafu;
use mod_info::ModInfo;
use skyrim_data_dir::Runtime;
use snafu::ResultExt as _;
use std::path::PathBuf;

/// Mod info readers
///
/// # glob samples
/// - Steam VFS: `D:/Steam/steamapps/common/Skyrim Special Edition/Data`
/// - MO2: `D:/GAME/ModOrganizer Skyrim SE/mods/*`
#[tauri::command]
pub(crate) fn load_mods_info(glob: &str, is_vfs_mode: bool) -> Result<Vec<ModInfo>, String> {
    let info = mod_info::get_all(glob, is_vfs_mode).or_else(|err| bail!(err))?;
    Ok(info)
}

/// Get skyrim se/vr directory.
///
/// e.g. `D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data`
#[tauri::command]
pub(crate) fn get_skyrim_data_dir(runtime: Runtime) -> Result<PathBuf, String> {
    match skyrim_data_dir::get_skyrim_data_dir(runtime).with_context(|_| NotFoundSkyrimDataDirSnafu)
    {
        Ok(path) => Ok(path),
        Err(err) => bail!(err),
    }
}
