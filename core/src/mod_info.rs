use crate::error::Result;
use glob::glob;
use indexmap::IndexMap;
use rayon::prelude::*;
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Remove `null` string
fn deserialize_remove_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
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
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_remove_null")]
    pub id: String,

    /// Mod name
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_remove_null")]
    pub name: String,

    /// Mod author
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_remove_null")]
    pub author: String,

    /// Mod download link
    #[serde(deserialize_with = "deserialize_remove_null")]
    #[serde(default)]
    pub site: String,

    /// TODO: Unknown
    #[serde(deserialize_with = "deserialize_remove_null")]
    #[serde(default)]
    pub auto: String,
}

/// - `mod_id`, `info.ini`
pub type ModsInfo = IndexMap<String, ModInfo>;
/// - `mod_id`, `priority`
pub type PriorityMap = HashMap<String, usize>;

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

    /// Use a priority map to arrange them in order of priority.
    /// # Note
    /// - If a key is not found in the priority map, it is removed.
    /// - If the priority map is empty, priority is determined alphabetically by id.
    fn sort_to_vec_by_priority(self, priority_map: PriorityMap) -> Vec<ModInfo>;
}

impl GetModsInfo for ModsInfo {
    fn get_all(pattern: &str) -> Result<ModsInfo> {
        let paths: Vec<PathBuf> = glob(pattern)?.filter_map(Result::ok).collect();

        let mut mod_info_map = paths
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
                                mod_info.id = id.clone();
                                (id, mod_info)
                            })
                    })
            })
            .collect::<Self>();
        mod_info_map.sort_keys();
        Ok(mod_info_map)
    }

    fn sort_to_vec_by_priority(mut self, priority_map: PriorityMap) -> Vec<ModInfo> {
        if priority_map.is_empty() {
            return self
                .into_iter()
                .map(|(id, mut mod_info)| {
                    mod_info.id = id;
                    mod_info
                })
                .collect();
        }

        let mut sorted_vec = priority_map
            .into_iter()
            .filter_map(|(id, priority)| self.swap_remove(&id).map(|mod_info| (priority, mod_info)))
            .collect::<Vec<_>>();

        sorted_vec.sort_by_key(|&(priority, _)| priority);
        sorted_vec
            .into_iter()
            .map(|(_, mod_info)| mod_info)
            .collect()
    }
}

/// Get `<id>` from `Nemesis_Engine/mods/<id>/info.ini`
#[inline]
fn extract_id_from_path(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .parent()
        .and_then(Path::file_name)
        .and_then(|os_str| os_str.to_str())
        .map(|s| s.to_string()) // String に変換
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[test]
    fn get_mod_info() -> Result<()> {
        let pattern = "../dummy/mods/*/info.ini";
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
        assert_eq_id("Nemesis_Engine/mods/test_mods/info.ini", Some("test_mod"));
        assert_eq_id("Nemesis_Engine/info.ini", Some("Nemesis_Engine"));
        assert_eq_id(r"Nemesis_Engine\mods\456\info.ini", Some("456"));
        assert_eq_id("info.ini", None);
    }

    #[test]
    fn test_sort_with_non() {
        let mods_info = [
            (
                "mod1".to_string(),
                ModInfo {
                    id: "mod1".to_string(),
                    name: "Mod 1".to_string(),
                    author: "Author 1".to_string(),
                    site: "Site 1".to_string(),
                    ..Default::default()
                },
            ),
            (
                "mod2".to_string(),
                ModInfo {
                    id: "mod2".to_string(),
                    name: "Mod 2".to_string(),
                    author: "Author 2".to_string(),
                    site: "Site 2".to_string(),
                    ..Default::default()
                },
            ),
            (
                "mod3".to_string(),
                ModInfo {
                    id: "mod3".to_string(),
                    name: "Mod 3".to_string(),
                    author: "Author 3".to_string(),
                    site: "Site 3".to_string(),
                    ..Default::default()
                },
            ),
        ];
        let mods_info_map: ModsInfo = mods_info.clone().into();

        // - If the priority map is empty, it is returned without sorting.
        assert_eq!(
            mods_info_map.sort_to_vec_by_priority(HashMap::new()),
            mods_info
                .into_iter()
                .map(|(_, mod_info)| mod_info)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_sort_with_priority_map() {
        let mods_info = [
            (
                "mod1".to_string(),
                ModInfo {
                    id: "mod1".to_string(),
                    name: "Mod 1".to_string(),
                    author: "Author 1".to_string(),
                    site: "Site 1".to_string(),
                    ..Default::default()
                },
            ),
            (
                "mod2".to_string(),
                ModInfo {
                    id: "mod2".to_string(),
                    name: "Mod 2".to_string(),
                    author: "Author 2".to_string(),
                    site: "Site 2".to_string(),
                    ..Default::default()
                },
            ),
            (
                "mod3".to_string(),
                ModInfo {
                    id: "mod3".to_string(),
                    name: "Mod 3".to_string(),
                    author: "Author 3".to_string(),
                    site: "Site 3".to_string(),
                    ..Default::default()
                },
            ),
        ];
        let mods_info_map: ModsInfo = mods_info.clone().into();

        // Sort by priority
        let mut priority_map = HashMap::new();
        priority_map.insert("mod1".to_string(), 2);
        priority_map.insert("mod2".to_string(), 1);
        priority_map.insert("mod3".to_string(), 3);
        let sorted = mods_info_map.sort_to_vec_by_priority(priority_map);

        let mut expected_mods_info_map: Vec<_> = mods_info
            .into_iter()
            .map(|(_, mod_info)| mod_info)
            .collect();
        expected_mods_info_map.swap(0, 1);

        assert_eq!(sorted, expected_mods_info_map);
    }
}
