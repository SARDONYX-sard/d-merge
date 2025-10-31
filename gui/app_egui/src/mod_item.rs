use mod_info::ModType;
use nemesis_merge::PatchMaps;
use rayon::{iter::Either, prelude::*};

/// Represents a single mod entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)] // Need hash to dnd
pub struct ModItem {
    /// Whether the mod is enabled.
    pub enabled: bool,
    /// - Nemesis/FNIS(vfs): e.g. `aaaa`
    /// - Nemesis(manual): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    /// - FNIS(manual): e.g. `<skyrim data dir>/meshes/actors/character/animations/aaaa`
    pub id: String,
    /// Display name of the mod.
    pub name: String,
    /// Mod source site.
    pub site: String,
    /// Load priority.
    pub priority: usize,

    /// Mod type. Nemesis, FNIS
    #[serde(default)]
    pub mod_type: ModType,
}

/// Columns that can be used for sorting mods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortColumn {
    Id,
    Name,
    ModType,
    Site,
    Priority,
}

/// Inherit `priority` & `enabled` from old list & Convert `ModInfo` into `ModItem`.
pub fn inherit_reorder_cast(old: &[ModItem], new: Vec<mod_info::ModInfo>) -> Vec<ModItem> {
    use std::collections::HashMap;

    if old.is_empty() {
        return from_mod_infos_with_incremented_priority(new);
    };

    let priority_map: HashMap<&str, (bool, usize)> = old
        .par_iter()
        .map(|item| (item.id.as_str(), (item.enabled, item.priority)))
        .collect();

    // NOTE: This relies on the fact that the priority is equal to the 1-based index of len.
    // Example
    // When len is 5 -> The last priority is 5.
    // start 5 + 1 = 6
    // The return value of fetch_add is the previous value, which is 6.
    let current_new_priority = std::sync::atomic::AtomicUsize::new(old.len() + 1);

    // Inherit priorities from old list
    let mut new: Vec<ModItem> = new
        .into_par_iter()
        .map(|item| {
            if let Some(&(enabled, priority)) = priority_map.get(item.id.as_str()) {
                ModItem {
                    enabled, // NOTE: Always set it to false during the fetch phase, then later check the existing active list and set it to true.
                    id: item.id,
                    name: item.name,
                    site: item.site,
                    priority,
                    mod_type: item.mod_type,
                }
            } else {
                ModItem {
                    enabled: false, // To prevent malfunctions, new mods are not enabled by default.
                    id: item.id,
                    name: item.name,
                    site: item.site,
                    priority: current_new_priority
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    mod_type: item.mod_type,
                }
            }
        })
        .collect();

    new.par_sort_unstable_by(|a, b| a.priority.cmp(&b.priority));
    new
}

/// Convert `ModInfo` into `ModItem`.
fn from_mod_infos_with_incremented_priority(mod_infos: Vec<mod_info::ModInfo>) -> Vec<ModItem> {
    mod_infos
        .into_par_iter()
        .enumerate()
        .map(|(i, item)| ModItem {
            enabled: false, // NOTE: Always set it to false during the fetch phase, then later check the existing active list and set it to true.
            id: item.id,
            name: item.name,
            site: item.site,
            priority: i,
            mod_type: item.mod_type,
        })
        .collect()
}

/// Convert `ModItem` into `PatchMaps`.
pub fn to_patches(skyrim_data_dir: &str, is_vfs: bool, mod_infos: &[ModItem]) -> PatchMaps {
    // - Nemesis/FNIS(vfs): e.g. `aaaa`
    // - Nemesis(manual): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    // - FNIS(manual): e.g. `<skyrim data dir>/meshes/actors/character/animations/aaaa`
    let (nemesis_entries, fnis_entries) = mod_infos
        .par_iter()
        .filter(|item| item.enabled)
        .partition_map(
            |ModItem {
                 id,
                 priority,
                 mod_type,
                 ..
             }| {
                let priority = *priority;

                match mod_type {
                    ModType::Nemesis => {
                        let id = if is_vfs {
                            format!("{skyrim_data_dir}/Nemesis_Engine/mod/{id}")
                        } else {
                            id.clone()
                        };
                        Either::Left((id, priority))
                    }
                    ModType::Fnis => {
                        let id = id.clone();
                        Either::Right((id, priority))
                    }
                }
            },
        );
    PatchMaps {
        nemesis_entries,
        fnis_entries,
    }
}
