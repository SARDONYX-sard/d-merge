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
#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortColumn {
    Id,
    Name,
    Site,
    Priority,
}

/// Convert `ModInfo` into `ModItem`.
pub fn from_mod_infos(mod_infos: Vec<mod_info::ModInfo>) -> Vec<ModItem> {
    mod_infos
        .into_par_iter()
        .enumerate()
        .map(|(i, mi)| ModItem {
            enabled: false, // NOTE: Always set it to false during the fetch phase, then later check the existing active list and set it to true.
            id: mi.id,
            name: mi.name,
            site: mi.site,
            priority: i,
            mod_type: mi.mod_type,
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
