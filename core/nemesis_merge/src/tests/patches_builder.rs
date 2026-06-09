use rapidhash::RapidHashSet as HashSet;
use rayon::{iter::Either, prelude::*};

use crate::{PatchMaps, PriorityMap};

/// Configuration for building [`PatchMaps`] in tests.
#[derive(Debug, Default)]
pub(crate) struct PatchMapsConfig<'a> {
    /// Glob pattern for the Skyrim Data directory (e.g. `"D:/MO2/mods/*"`)
    pub pattern: &'a str,
    /// Whether to use the virtual filesystem
    pub use_vfs: bool,
    /// Nemesis mod names to exclude (empty = exclude none)
    pub nemesis_excludes: &'a [&'a str],
    /// FNIS mod namespaces to exclude (empty = exclude none)
    pub fnis_excludes: &'a [&'a str],
}

/// Build [`PatchMaps`] from a mod directory pattern for use in tests.
pub(crate) fn build_patch_maps(config: PatchMapsConfig<'_>) -> PatchMaps {
    use mod_info::ModType;

    let PatchMapsConfig { pattern, use_vfs, nemesis_excludes, fnis_excludes } = config;

    let mod_list = mod_info::get_all(pattern, use_vfs).unwrap();
    let nemesis_excludes: HashSet<&str> = nemesis_excludes.iter().copied().collect();
    let fnis_excludes: HashSet<&str> = fnis_excludes.iter().copied().collect();

    let (nemesis_entries, fnis_entries): (PriorityMap, PriorityMap) = mod_list
        .par_iter()
        .enumerate()
        .filter_map(|(idx, info)| {
            let mod_id = std::path::Path::new(&info.id)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&info.id);

            match info.mod_type {
                ModType::Nemesis | ModType::NemesisExt if !nemesis_excludes.contains(mod_id) => {
                    Some(Either::Left((info.id.clone(), idx)))
                }
                ModType::Fnis if !fnis_excludes.contains(mod_id) => {
                    Some(Either::Right((info.id.clone(), idx)))
                }
                _ => None,
            }
        })
        .partition_map(|e| e);

    PatchMaps { nemesis_entries, fnis_entries }
}
