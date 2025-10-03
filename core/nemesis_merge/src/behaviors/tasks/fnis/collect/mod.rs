pub mod owned;

use crate::{
    behaviors::tasks::fnis::{
        collect::owned::{collect_fnis_injection, OwnedFnisInjection},
        patch_gen::generated_behaviors::{
            BehaviorEntry, AUXBONES, CREATURES, HUMANOID, PLANTS_ACTIVATORS, SKELETONS,
        },
    },
    errors::Error,
    PriorityMap,
};
use std::path::PathBuf;

/// Collect FNIS injections by scanning specific directories under `<skyrim_data_dir>/meshes`.
///
/// It searches for `meshes/**/animations/<namespace>/FNIS_*_List.txt` under:
pub fn collect_all_fnis_injections(
    skyrim_data_dir: &str,
    fnis_entries: &PriorityMap,
) -> (Vec<OwnedFnisInjection>, Vec<Error>) {
    use std::sync::atomic::Ordering;

    const ALL_MAPS: &[&phf::Map<&str, BehaviorEntry>] = &[
        &HUMANOID,
        &CREATURES,
        &SKELETONS,
        &AUXBONES,
        &PLANTS_ACTIVATORS,
    ];

    // Flag to ensure "draugr" from CREATURES is only processed once
    let draugr_skipped = std::sync::atomic::AtomicBool::new(false);

    let results: Vec<_> = ALL_MAPS
        .iter()
        .flat_map(|map| {
            map.values().flat_map(|entry| {
                // Special case: skip CREATURES' draugr if it was already processed by SKELETONS
                let skip =
                    entry.behavior_object == "draugr" && draugr_skipped.load(Ordering::Relaxed);
                if !skip && entry.behavior_object == "draugr" {
                    draugr_skipped.store(true, Ordering::Relaxed);
                }

                // Mark draugr as processed if we encounter it in SKELETONS
                if entry.behavior_object == "draugr" {
                    draugr_skipped.store(true, Ordering::Relaxed);
                }

                let base_dir = entry.base_dir;
                let glob_pat = format!("{skyrim_data_dir}/meshes/{base_dir}/animations/*");

                collect_paths(&glob_pat)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(move |ns_path| ns_path.is_dir() && !skip) // skip draugr from CREATURES
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
        })
        .collect();
    let (oks, errs): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
    let oks = oks.into_iter().map(Result::unwrap).collect();
    let errs = errs.into_iter().map(Result::unwrap_err).collect();

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
    use rayon::prelude::*;

    #[test]
    #[ignore = "local only"]
    fn test_parse_relative_path() {
        crate::global_logger::global_logger("./test.log", tracing::Level::TRACE).unwrap();

        let fnis_entries = ["BiS_WashMe", "FNISFlyer", "FNISZoo", "XPMSE", "TKDodge"]
            .into_par_iter()
            .enumerate()
            .map(|(idx, namespace)| (namespace.to_string(), idx))
            .collect();
        let res = collect_all_fnis_injections("../../dummy/fnis_test_mods/*", &fnis_entries);

        std::fs::write("./result.log", format!("{res:#?}")).unwrap();
    }
}
