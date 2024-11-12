pub mod error;

use crate::error::Result;
use glob::glob;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

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
}

/// - `mod_id`, `info.ini`
pub type ModsInfo = Vec<ModInfo>;

pub trait GetModsInfo {
    /// Get `info.ini` and store it in a [`IndexMap`] using `<id>` as the key.
    /// - Intended `Nemesis_Engine/mods/<id>/info.ini`
    ///
    /// # Errors
    /// If invalid glob pattern.
    ///
    /// # Note
    /// Priority and `id` cannot be obtained at this stage.
    fn get_all(pattern: &str) -> Result<ModsInfo>;
}

impl GetModsInfo for ModsInfo {
    fn get_all(pattern: &str) -> Result<ModsInfo> {
        let paths: Vec<PathBuf> = glob(pattern)?.filter_map(Result::ok).collect();

        let mod_info = paths
            .par_iter()
            .filter_map(|path| {
                path.exists()
                    .then(|| extract_id_from_path(path))
                    .flatten()
                    .and_then(|id| {
                        fs::read_to_string(path)
                            .ok()
                            .and_then(|contents| serde_ini::from_str(&contents).ok())
                            .map(|mut mod_info: ModInfo| {
                                mod_info.id = id;
                                mod_info
                            })
                    })
            })
            .collect::<Self>();
        Ok(mod_info)
    }
}

/// Get `<id>` from `Nemesis_Engine/mods/<id>/info.ini`
#[inline]
fn extract_id_from_path(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .parent()
        .and_then(Path::file_name)
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[test]
    fn get_mod_info() -> Result<()> {
        let pattern = "../../dummy/Data/Nemesis_Engine/mod/*/info.ini";
        let info = ModsInfo::get_all(pattern)?;
        println!("{:#?}", info);
        Ok(())
    }

    #[test]
    fn test_extract_id_from_path() {
        fn assert_eq_id(path: impl AsRef<Path>, id: Option<&str>) {
            assert_eq!(extract_id_from_path(path), id.map(|s| s.to_string()));
        }

        assert_eq_id("Nemesis_Engine/mods/123/info.ini", Some("123"));
        assert_eq_id("Nemesis_Engine/mods/test_id/info.ini", Some("test_id"));
        assert_eq_id("Nemesis_Engine/info.ini", Some("Nemesis_Engine"));
        // assert_eq_id(r"Nemesis_Engine\mods\456\info.ini", Some("456")); // <- windows only
        assert_eq_id("info.ini", None);
    }
}
