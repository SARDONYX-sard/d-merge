pub mod owned;

use crate::{
    behaviors::tasks::fnis::collect::owned::{collect_fnis_injection, OwnedFnisInjection},
    PriorityMap,
};
use rayon::prelude::*;
use std::path::PathBuf;

/// Collect FNIS injections by scanning specific directories under `<skyrim_data_dir>/meshes`.
///
/// It searches for `animations/<namespace>/FNIS_*_List.txt` under:
/// - `meshes/actors/*/`
/// - `meshes/actors/ambient/chicken/`
/// - `meshes/actors/character/_1stperson/`
/// - `meshes/auxbones/tail/`
/// - `meshes/dlc01/plants/caveworm/`
/// - `meshes/dlc01/plants/cavewormgroup/`
/// - `meshes/dlc01/plants/cavewormsmall/`
pub fn collect_all_fnis_injections(
    skyrim_data_dir: &str,
    fnis_entries: &PriorityMap,
) -> Vec<OwnedFnisInjection> {
    const TARGET_PATTERNS: &[&str] = &[
        "meshes/actors/*",
        "meshes/actors/ambient/chicken",
        "meshes/actors/character/_1stperson",
        "meshes/auxbones/tail",
        "meshes/dlc01/plants/caveworm",
        "meshes/dlc01/plants/cavewormgroup",
        "meshes/dlc01/plants/cavewormsmall",
    ];

    // Resolve data directories (single path or glob)
    let mut data_dirs: Vec<PathBuf> = Vec::new();
    if skyrim_data_dir.contains(['*', '?', '[']) {
        match collect_paths(skyrim_data_dir) {
            Ok(paths) => data_dirs.par_extend(paths),
            Err(e) => {
                tracing::warn!(
                    "Invalid glob pattern for skyrim_data_dir: {skyrim_data_dir}, error={e}"
                );
                return vec![];
            }
        }
    } else {
        data_dirs.push(PathBuf::from(skyrim_data_dir));
    }

    data_dirs
        .iter()
        .flat_map(|data_dir| {
            TARGET_PATTERNS
                .iter()
                .flat_map(|pattern| {
                    let search_root = data_dir.join(pattern);
                    let glob_pat = format!("{}/animations/*", search_root.display());

                    collect_paths(&glob_pat)
                        .unwrap_or_else(|e| {
                            tracing::warn!("Glob error at {glob_pat}: {e}");
                            vec![]
                        })
                        .into_iter()
                        .filter(|ns_path| ns_path.is_dir())
                        .filter_map(|ns_path| {
                            let namespace = ns_path.file_name()?.to_string_lossy();
                            let priority = fnis_entries.get(namespace.as_ref()).copied()?;

                            match collect_fnis_injection(&ns_path, &namespace,priority) {
                                Ok(injection) => Some(injection),
                                Err(e) => {
                                    tracing::warn!(namespace=%namespace, error=%e, "Failed to collect FNIS injection");
                                    None
                                }
                            }
                        })
                })
        })
        .collect::<Vec<_>>()
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
