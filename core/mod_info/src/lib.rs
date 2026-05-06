pub mod error;

use std::{
    fs,
    path::{Path, PathBuf},
};

use rayon::prelude::*;

use crate::error::Error;

/// Collect both Nemesis and FNIS mods into a single vector.
///
/// # Nemesis
/// | is_vfs | glob pattern                                               | id extracted as                   |
/// |--------|------------------------------------------------------------|-----------------------------------|
/// | true   | `{skyrim_data_dir}/Nemesis_Engine/mod/*/info.ini`         | `<id>` from `Nemesis_Engine/mod/<id>/info.ini` |
/// | false  | `{skyrim_data_dir}/Nemesis_Engine/mod/*/info.ini`         | full parent path (e.g. `MO2/mod/mod_name/meshes/.../Nemesis_Engine/mod/aaaa`) |
///
/// # NemesisExt
/// | is_vfs | glob pattern                                               | id extracted as                   |
/// |--------|------------------------------------------------------------|-----------------------------------|
/// | true   | `{skyrim_data_dir}/Nemesis_EngineExt/mod/*/info.ini`         | `<id>` from `Nemesis_Engine/mod/<id>/info.ini` |
/// | false  | `{skyrim_data_dir}/Nemesis_EngineExt/mod/*/info.ini`         | full parent path (e.g. `MO2/mod/mod_name/meshes/.../Nemesis_Engine/mod/aaaa`) |
///
/// # FNIS
/// | is_vfs | glob pattern                                                         | id extracted as                   |
/// |--------|----------------------------------------------------------------------|-----------------------------------|
/// |  any   | `{skyrim_data_dir}/meshes/actors/character/animations/*/FNIS_*_List.txt` | `<id>` from `animations/<id>`     |
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
    mods.par_extend(get_all_nemesis_ext(skyrim_data_dir, is_vfs)?);
    mods.par_extend(get_all_fnis(skyrim_data_dir)?);
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

    let mods = jwalk_glob::glob_files(&nemesis_pattern)
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

fn get_all_nemesis_ext(skyrim_data_dir: &str, is_vfs: bool) -> Result<Vec<ModInfo>, Error> {
    let nemesis_pattern = format!("{skyrim_data_dir}/Nemesis_EngineExt/mod/*/info.ini");

    let mods = jwalk_glob::glob_files(&nemesis_pattern)
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
            let mut mod_info = read_mod_info(path, id)?;
            mod_info.mod_type = ModType::NemesisExt;

            Some(mod_info)
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

/// FNIS: `<skyrim_data_dir>/meshes/actors/character/animations/*/FNIS_*_List.txt`
///
/// # Errors
/// If invalid glob pattern.
fn get_all_fnis(skyrim_data_dir: &str) -> Result<Vec<ModInfo>, Error> {
    use std::collections::HashSet;

    fn collect_from_fnis_list(fnis_list_pattern: &str) -> Result<HashSet<ModInfo>, Error> {
        let mods = jwalk_glob::glob_files(fnis_list_pattern)
            .par_iter()
            .filter_map(|path| {
                if !path.exists() {
                    return None;
                }
                // <skyrim_data_dir>/meshes/**/animations/<vfs_id>
                let parent_dir = path.parent()?;

                // get `<vfs_id>`
                let name = parent_dir.file_name()?.display().to_string();
                let id = name.clone();

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

    // TkDodgeSE lacks FNIS_*_List.txt and consists solely of animations, making acquisition difficult.
    // It would be easier to distribute the FNIS patch separately as a Nemesis patch.
    let fnis_list_pattern = format!("{skyrim_data_dir}/meshes/**/animations/*/FNIS_*_List.txt");
    let mut mods: Vec<_> = collect_from_fnis_list(&fnis_list_pattern)?
        .into_par_iter()
        .collect();
    mods.par_sort_unstable_by(|a, b| a.name.cmp(&b.name));

    Ok(mods)
}

/// # Note
/// - Intended `Nemesis_Engine/mods/<id>/info.ini`
/// - `priority`: As with MO2, lower numbers indicate lower priority, higher numbers indicate higher priority.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ModInfo {
    /// Mod-specific dir name.
    /// - Nemesis/FNIS(vfs): e.g. `aaaa`
    /// - Nemesis(manual): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    /// - FNIS(manual): e.g. `<skyrim data dir>/meshes/actors/character/animations/aaaa`
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub id: String,

    /// Mod name
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub name: String,

    /// Mod author
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub author: String,

    /// Mod download link
    #[serde(default, deserialize_with = "deserialize_site")]
    pub site: String,

    /// TODO: Unknown
    #[serde(default, deserialize_with = "deserialize_remove_null")]
    pub auto: String,

    /// Mod type. Nemesis, FNIS
    #[serde(default)]
    pub mod_type: ModType,
}

// NOTE: Order follows the sequence in which the tools were created. The first one created was FNIS.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModType {
    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/meshes/actors/character/animations/aaaa`
    Fnis,
    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    #[default]
    Nemesis,

    /// GUI developers must add the following to the paths array in `nemesis_merge::behavior_gen`.
    /// - `<skyrim data dir>/Nemesis_EngineExt/mod/aaaa`
    ///
    /// A patch for the exact path from meshes (note that the file extension is .bin, not .hkx)
    /// - e.g., `Nemesis_EngineExt/mod/aaaa/meshes/actors/troll/characters/troll.bin/#0029.txt`
    NemesisExt,
}

impl ModType {
    /// Get the `&static str` corresponding to `ModType`
    ///
    /// # Examples
    ///
    /// ```rust
    /// assert_eq!(mod_info::ModType::Nemesis.as_str(), "Nemesis");
    /// assert_eq!(mod_info::ModType::Fnis.as_str(), "FNIS");
    /// ```
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Nemesis => "Nemesis",
            Self::NemesisExt => "NemesisExt",
            Self::Fnis => "FNIS",
        }
    }
}

/// Remove `null` string
fn deserialize_remove_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: std::borrow::Cow<'de, str> = serde::Deserialize::deserialize(deserializer)?;
    Ok(if s.eq_ignore_ascii_case("null") {
        String::new() // 0 alloc
    } else {
        s.to_string()
    })
}

/// Remove "null" and prepend "https://" if starts with "www."
fn deserialize_site<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: std::borrow::Cow<'de, str> = serde::Deserialize::deserialize(deserializer)?;
    if s.eq_ignore_ascii_case("null") || s.trim().is_empty() {
        Ok(String::new())
    } else if s.starts_with("www.") {
        Ok(format!("https://{s}"))
    } else {
        Ok(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use pretty_assertions::assert_eq;

    use super::*;

    #[ignore = "Requires actual Skyrim Data directory with Nemesis mods"]
    #[test]
    fn get_mod_info() {
        let pattern = "D:/GAME/ModOrganizer Skyrim SE/mods/*";
        let info = get_all(pattern, false).unwrap();

        dbg!(info.len());
        std::fs::create_dir_all("../../dummy").unwrap();
        std::fs::write(
            "../../dummy/debug/get_all_mods_info.log",
            format!("{info:#?}"),
        )
        .unwrap();
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
