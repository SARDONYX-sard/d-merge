use std::path::PathBuf;

use mod_info::{ModInfo, ModType};
use nemesis_merge::PatchMaps;
use rayon::{iter::Either, prelude::*};

/// Represents a single mod entry.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)] // Need hash to dnd
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

    // We can just collect this data as needed. There’s no need to save it in the settings.
    //
    /// Absolute paths to `FNIS_*_List.txt` files for this mod.
    /// Only populated for `ModType::Fnis`.
    /// Multiple entries occur when a mod targets multiple creatures
    /// (e.g. `C:/steamapps/Skyrim Special Edition/.../FNIS_FNISZoo_dog_List.txt`).
    #[serde(skip)]
    pub fnis_list_paths: Vec<PathBuf>,
}

/// Inherit priority/enabled from old list.
/// New items are appended in alphabetical order.
/// Final priorities are always normalized.
pub fn inherit_reorder_cast(old: &[ModItem], new: Vec<ModInfo>) -> Vec<ModItem> {
    let old_map: rapidhash::fast::RapidHashMap<&str, (bool, usize)> =
        old.par_iter().map(|m| (m.id.as_str(), (m.enabled, m.priority))).collect();

    let (mut with_old, mut without_old): (Vec<ModItem>, Vec<ModItem>) =
        new.into_par_iter().partition_map(|info| {
            let ModInfo { id, name, site, mod_type, fnis_list_paths, .. } = info;

            if let Some(&(enabled, priority)) = old_map.get(id.as_str()) {
                Either::Left(ModItem {
                    enabled,
                    id,
                    name,
                    site,
                    priority,
                    mod_type,
                    fnis_list_paths,
                })
            } else {
                Either::Right(ModItem {
                    enabled: false,
                    id,
                    name,
                    site,
                    priority: 0, // temporary
                    mod_type,
                    ..Default::default()
                })
            }
        });

    rayon::join(
        // Existing mods: order by old priority
        || with_old.par_sort_unstable_by(|a, b| a.priority.cmp(&b.priority)),
        // New mods: alphabetical
        || without_old.par_sort_unstable_by(|a, b| a.id.cmp(&b.id)),
    );
    with_old.par_extend(without_old);

    // Final deterministic normalization
    with_old.par_iter_mut().enumerate().for_each(|(index, item)| item.priority = index);

    with_old
}

/// Convert `ModItem` into `PatchMaps`.
pub fn to_patches(skyrim_data_dir: &str, is_vfs: bool, mod_infos: &[ModItem]) -> PatchMaps {
    // - Nemesis/FNIS(vfs): e.g. `aaaa`
    // - Nemesis(manual): e.g. `<skyrim data dir>/Nemesis_Engine/mod/aaaa`
    // - FNIS(manual): e.g. `<skyrim data dir>/meshes/actors/character/animations/aaaa/FNIS_aaaa_List.txt`
    let (nemesis_entries, fnis_groups): (_, Vec<Vec<(String, usize)>>) =
        mod_infos.par_iter().filter(|item| item.enabled).partition_map(|mod_item| {
            let ModItem { id, priority, mod_type, .. } = mod_item;
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
                ModType::NemesisExt => {
                    let id = if is_vfs {
                        format!("{skyrim_data_dir}/Nemesis_EngineExt/mod/{id}")
                    } else {
                        id.clone()
                    };
                    Either::Left((id, priority))
                }
                ModType::Fnis => Either::Right(fnis_list_entries(mod_item, priority)),
            }
        });

    let fnis_entries = fnis_groups.into_par_iter().flatten().collect();
    PatchMaps { nemesis_entries, fnis_entries }
}

/// Expand a single FNIS [`ModItem`] into one `(id, priority)` pair per `List.txt` path.
///
/// `id` is the absolute path to each `FNIS_*_List.txt` file.
/// All entries share the same `priority` since they belong to the same mod.
fn fnis_list_entries(mod_item: &ModItem, priority: usize) -> Vec<(String, usize)> {
    mod_item.fnis_list_paths.iter().map(|p| (p.display().to_string(), priority)).collect()
}

/// Reorder priorities by mod type:
/// Nemesis -> NemesisExt -> FNIS
///
/// Inside each mod type, mods are ordered by `id` alphabetically.
/// After sorting, priorities are reassigned sequentially starting from 0.
pub fn reorder_mods_priorities(mods: &mut [ModItem]) {
    use mod_info::ModType;

    const fn mod_type_rank(mod_type: &ModType) -> u8 {
        match mod_type {
            ModType::Nemesis => 0,
            ModType::NemesisExt => 1,
            ModType::Fnis => 2,
        }
    }

    mods.par_sort_unstable_by(|a, b| {
        mod_type_rank(&a.mod_type).cmp(&mod_type_rank(&b.mod_type)).then_with(|| a.id.cmp(&b.id))
    });
    mods.par_iter_mut().enumerate().for_each(|(priority, item)| {
        item.priority = priority;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_gaps_and_non_zero_start() {
        let old = vec![
            ModItem {
                enabled: true,
                id: "b".into(),
                name: "B".into(),
                site: "x".into(),
                priority: 10,
                mod_type: ModType::Nemesis,
                ..Default::default()
            },
            ModItem {
                enabled: true,
                id: "a".into(),
                name: "A".into(),
                site: "x".into(),
                priority: 3,
                mod_type: ModType::Nemesis,
                ..Default::default()
            },
        ];

        let new = vec![
            ModInfo {
                id: "a".into(),
                name: "A".into(),
                site: "x".into(),
                mod_type: ModType::Nemesis,
                ..Default::default()
            },
            ModInfo {
                id: "b".into(),
                name: "B".into(),
                site: "x".into(),
                mod_type: ModType::Fnis,
                ..Default::default()
            },
            ModInfo {
                id: "c".into(),
                name: "C".into(),
                site: "x".into(),
                mod_type: ModType::Fnis,
                ..Default::default()
            },
        ];

        let result = inherit_reorder_cast(&old, new);

        let priorities: Vec<usize> = result.iter().map(|m| m.priority).collect();
        assert_eq!(priorities, vec![0, 1, 2]);

        let ids: Vec<&str> = result.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn preserves_relative_order_of_existing_mods() {
        let old = vec![
            ModItem {
                enabled: true,
                id: "low".into(),
                name: "Low".into(),
                site: "x".into(),
                priority: 1,
                ..Default::default()
            },
            ModItem {
                enabled: true,
                id: "high".into(),
                name: "High".into(),
                site: "x".into(),
                priority: 100,
                ..Default::default()
            },
        ];

        let new = vec![
            ModInfo {
                id: "high".into(),
                name: "High".into(),
                site: "x".into(),
                ..Default::default()
            },
            ModInfo {
                id: "low".into(),
                name: "Low".into(),
                site: "x".into(),
                ..Default::default()
            },
        ];

        let result = inherit_reorder_cast(&old, new);

        let ids: Vec<&str> = result.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["low", "high"]);
    }
}
