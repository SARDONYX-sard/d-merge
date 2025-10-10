pub mod owned;

use crate::behaviors::tasks::fnis::collect::owned::{collect_fnis_injection, OwnedFnisInjection};
use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::{
    AUXBONES, CREATURES, HUMANOID, PLANTS_ACTIVATORS, SKELETONS,
};
use crate::errors::Error;
use crate::PriorityMap;
use rayon::prelude::*;
use std::path::PathBuf;

/// Collect FNIS injections by scanning specific directories under `<skyrim_data_dir>/meshes`.
///
/// It searches for `meshes/**/animations/<namespace>/FNIS_*_List.txt` under:
pub fn collect_all_fnis_injections(
    skyrim_data_dir: &str,
    fnis_entries: &PriorityMap,
) -> (Vec<OwnedFnisInjection>, Vec<Error>) {
    let all_maps = HUMANOID
        .values()
        .chain(CREATURES.values())
        .chain(
            SKELETONS
                .values()
                // Flag to ensure "draugr" from CREATURES is only processed once
                .filter(|entry| entry.behavior_object != "draugr"),
        )
        .chain(AUXBONES.values())
        .chain(PLANTS_ACTIVATORS.values());

    let (oks, errs): (Vec<_>, Vec<_>) = all_maps
        .par_bridge()
        .flat_map(|entry| {
            let glob_pattern = format!("{skyrim_data_dir}/meshes/{}/animations/*", entry.base_dir);

            collect_paths(&glob_pattern)
                .unwrap_or_default()
                .into_par_iter()
                .filter(move |ns_path| ns_path.is_dir()) // skip draugr from CREATURES
                .filter_map(|ns_path| {
                    let namespace = ns_path.file_name()?.to_string_lossy().to_string();
                    let priority = fnis_entries.get(namespace.as_str()).copied()?;
                    Some((ns_path, namespace, priority))
                })
                .map(|(ns_path, namespace, priority)| {
                    Ok(collect_fnis_injection(
                        &ns_path, entry, &namespace, priority,
                    )?)
                })
        })
        .partition_map(|res| match res {
            Ok(ok) => rayon::iter::Either::Left(ok),
            Err(err) => rayon::iter::Either::Right(err),
        });

    (oks, errs)
}

/// Collect case-insensitive paths using glob.
///
/// # Errors
/// If invalid glob pattern.
pub fn collect_paths(pattern: &str) -> Result<Vec<PathBuf>, glob::PatternError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "local only"]
    fn test_parse_relative_path() {
        let output_path = "../../dummy/debug/collect_all_fnis_injections.log";

        let fnis_entries = ["BiS_WashMe", "FNISFlyer", "FNISZoo", "XPMSE", "TKDodge"]
            .into_par_iter()
            .enumerate()
            .map(|(idx, namespace)| (namespace.to_string(), idx))
            .collect();
        let res = collect_all_fnis_injections("../../dummy/fnis_test_mods/*", &fnis_entries);

        std::fs::write(output_path, format!("{res:#?}")).unwrap();
    }
}
