pub mod error;

use crate::error::Error;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

/// Collect both Nemesis and FNIS mods into a single vector.
///
/// # Nemesis
/// | is_vfs | glob pattern                                               | id extracted as                   |
/// |--------|------------------------------------------------------------|-----------------------------------|
/// | true   | `{skyrim_data_dir}/Nemesis_Engine/mod/*/info.ini`         | `<id>` from `Nemesis_Engine/mod/<id>/info.ini` |
/// | false  | `{skyrim_data_dir}/Nemesis_Engine/mod/*/info.ini`         | full parent path (e.g. `MO2/mod/mod_name/meshes/.../Nemesis_Engine/mod/aaaa`) |
///
/// # FNIS
/// | is_vfs | glob pattern                                                         | id extracted as                   |
/// |--------|----------------------------------------------------------------------|-----------------------------------|
/// | true   | `{skyrim_data_dir}/meshes/actors/character/animations/*/FNIS_*_List.txt` | `<id>` from `animations/<id>`     |
/// | false  | `{skyrim_data_dir}/meshes/actors/character/animations/*/FNIS_*_List.txt` | full parent path (e.g. `MO2/mods/mod_name/.../animations/aaaa`) |
///
/// - `is_vfs`:
///   Whether the lookup is done in the virtualized `Data` directory (true) or
///   directly under the mods directory (false).
///
/// # Errors
/// Returns [`Error`] if glob expansion fails or files cannot be read.
///
/// # Examples
/// ```no_run
/// use mod_info::get_all_mods;
///
/// // VFS mode(real Skyrim Data dir)
/// let mods_vfs = get_all_mods("C:/Games/Skyrim Special Edition/Data", true)?;
///
/// // Manual mode(MO2-managed mods dir)
/// let mods_mo2 = get_all_mods("C:/Modding/MO2/mods/*", false)?;
/// ```
pub fn get_all(skyrim_data_dir: &str, is_vfs: bool) -> Result<Vec<ModInfo>, Error> {
    let mut mods = Vec::new();
    mods.par_extend(get_all_nemesis(skyrim_data_dir, is_vfs)?);
    mods.par_extend(get_all_fnis(skyrim_data_dir, is_vfs)?);
    Ok(mods)
}

/// Get `info.ini` and store it in a [`IndexMap`] using `<id>` as the key.
///
/// - `is_vfs` is true:
///   - glob `<skyrim data dir>/Nemesis_Engine/mods/*/info.ini`
///   - id: `Nemesis_Engine/mods/<id>/info.ini` e.g. `aaaa`
///
/// - `is_vfs` is false:
///   - glob: e.g. `<MO2 exe dir>/mods/*/Nemesis_Engine/mods/*/info.ini`
///   - id: (e.g.: `MO2/mods/mod_name/meshes/[...]/Nemesis_Engine/mods/aaaa`)
///
/// # Errors
/// If invalid glob pattern.
///
/// # Note
/// Priority and `id` cannot be obtained at this stage.
fn get_all_nemesis(skyrim_data_dir: &str, is_vfs: bool) -> Result<Vec<ModInfo>, Error> {
    let nemesis_pattern = format!("{skyrim_data_dir}/Nemesis_Engine/mod/*/info.ini");

    let mods = collect_paths(&nemesis_pattern)?
        .par_iter()
        .filter_map(|path| {
            if !path.exists() {
                return None;
            }
            let id = if is_vfs {
                extract_nemesis_id_from_path(path)?.to_string()
            } else {
                path.parent().unwrap_or(path).display().to_string()
            };
            read_mod_info(path, id)
        })
        .collect();

    Ok(mods)
}

fn read_mod_info(path: &PathBuf, id: String) -> Option<ModInfo> {
    let contents = fs::read_to_string(path).ok()?;
    let mut mod_info: ModInfo = serde_ini::from_str(&contents).ok()?;
    mod_info.id = id;
    Some(mod_info)
}

/// Get `<id>` from `Nemesis_Engine/mods/<id>/info.ini`
fn extract_nemesis_id_from_path(path: &Path) -> Option<&str> {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|os_str| os_str.to_str())
}

/// Collect case insensitive paths
///
/// # Errors
/// If invalid glob pattern.
pub fn collect_paths(pattern: &str) -> Result<Vec<PathBuf>, Error> {
    Ok(glob::glob_with(
        pattern,
        glob::MatchOptions {
            case_sensitive: false, // To support Linux
            require_literal_separator: false,
            require_literal_leading_dot: false,
        },
    )?
    .filter_map(Result::ok)
    .collect())
}

// format!("{animations fnis mod dir}/FNIS_*_List.txt");

/// FNIS: `<skyrim_data_dir>/meshes/actors/character/animations/*/FNIS_*_List.txt`
///
/// # Errors
/// If invalid glob pattern.
fn get_all_fnis(skyrim_data_dir: &str, is_vfs: bool) -> Result<Vec<ModInfo>, Error> {
    let fnis_pattern =
        format!("{skyrim_data_dir}/meshes/actors/character/animations/*/FNIS_*_List.txt");

    let mods = collect_paths(&fnis_pattern)?
        .par_iter()
        .filter_map(|path| {
            if !path.exists() {
                return None;
            }
            let parent_dir = path.parent()?;

            // get `<vfs_id>` from animations/<vfs_id>/FNIS_*_List.txt
            let name = parent_dir.file_name()?.display().to_string();

            let id = if is_vfs {
                name.clone()
            } else {
                parent_dir.display().to_string()
            };

            let mod_info = ModInfo {
                id,
                name,
                mod_type: ModType::Fnis,
                ..Default::default()
            };
            Some(mod_info)
        })
        .collect();

    Ok(mods)
}

/// # Note
/// - Intended `Nemesis_Engine/mods/<id>/info.ini`
/// - `priority`: As with MO2, lower numbers indicate lower priority, higher numbers indicate higher priority.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModInfo {
    /// Mod-specific dir name.
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub id: String,

    /// Mod name
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub name: String,

    /// Mod author
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub author: String,

    /// Mod download link
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub site: String,

    /// TODO: Unknown
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub auto: String,

    /// For FNIS mods, the path to the list is entered in this field.
    ///
    /// This enables FNIS mod detection for this mod from this path.
    #[serde(default)]
    pub mod_type: ModType,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModType {
    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    #[default]
    Nemesis,
    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/meshes/.../animations/aaaa`
    Fnis,
}

/// Remove `null` string
fn deserialize_remove_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    use std::borrow::Cow;

    let s = Cow::<'de, str>::deserialize(deserializer)?;
    Ok(match s.as_ref() {
        "null" => String::new(), // 0 alloc
        _ => s.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[test]
    fn get_mod_info() -> Result<(), Error> {
        let pattern = "D:/GAME/ModOrganizer Skyrim SE/mods/*";
        let info = get_all(pattern, false)?;
        println!("{info:#?}");
        Ok(())
    }

    #[test]
    fn test_extract_id_from_path() {
        fn assert_eq_id(path: impl AsRef<Path>, id: Option<&str>) {
            let path = path.as_ref();
            assert_eq!(extract_nemesis_id_from_path(path), id);
        }

        assert_eq_id("Nemesis_Engine/mods/123/info.ini", Some("123"));
        assert_eq_id("Nemesis_Engine/mods/test_id/info.ini", Some("test_id"));
        assert_eq_id("Nemesis_Engine/info.ini", Some("Nemesis_Engine"));
        // assert_eq_id(r"Nemesis_Engine\mods\456\info.ini", Some("456")); // <- windows only
        assert_eq_id("info.ini", None);
    }
}
