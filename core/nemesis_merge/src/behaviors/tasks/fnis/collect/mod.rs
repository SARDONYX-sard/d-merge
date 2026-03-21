mod glob;
pub mod owned;

use crate::behaviors::tasks::fnis::collect::owned::{collect_fnis_injection, OwnedFnisInjection};
use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::{
    BehaviorEntry, AUXBONES, CREATURES, HUMANOID, PLANTS_ACTIVATORS, SKELETONS,
};
use crate::errors::Error;
use crate::PriorityMap;
use std::path::PathBuf;
use std::sync::LazyLock;

static ALL_ENTRIES: LazyLock<Vec<&'static BehaviorEntry>> = LazyLock::new(|| {
    HUMANOID
        .values()
        .chain(CREATURES.values())
        .chain(SKELETONS.values().filter(|e| e.behavior_object != "draugr"))
        .chain(AUXBONES.values())
        .chain(PLANTS_ACTIVATORS.values())
        .collect()
});

/// Expands `skyrim_data_dir` into concrete directory paths.
///
/// Supports:
/// - Literal paths: `C:/Skyrim/Data`
/// - Trailing glob: `mods/*`
/// - Mid-path glob: `mods/*/Data`
/// - Partial name: `mods/FNIS*/Data`
/// - Recursive: `mods/**/Data`
fn expand_data_dirs(skyrim_data_dir: &str) -> Vec<PathBuf> {
    let has_glob = skyrim_data_dir.contains(['*', '?', '[']);
    if !has_glob {
        return vec![PathBuf::from(skyrim_data_dir)];
    }

    // Split into literal root (before first glob component) and the full pattern.
    let path = std::path::Path::new(skyrim_data_dir);
    let mut root = PathBuf::new();
    let mut found_glob = false;
    for component in path.components() {
        let s = component.as_os_str().to_str().unwrap_or("");
        if !found_glob && !s.contains(['*', '?', '[']) {
            root.push(component);
        } else {
            found_glob = true;
        }
    }

    // Determine max walk depth from pattern components after root.
    // `**` can match arbitrary depth, so use usize::MAX in that case.
    let pat_after_root = std::path::Path::new(skyrim_data_dir)
        .strip_prefix(&root)
        .unwrap_or_else(|_| std::path::Path::new(skyrim_data_dir));
    let has_recursive = pat_after_root.components().any(|c| c.as_os_str() == "**");
    let pat_depth = pat_after_root.components().count();
    let max_depth = if has_recursive { usize::MAX } else { pat_depth };

    // The pattern to match against paths relative to root's parent,
    // since jwalk includes root itself at depth 0.
    jwalk::WalkDir::new(&root)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|r| {
            r.map_err(|_e| {
                #[cfg(feature = "tracing")]
                tracing::warn!(error = %_e, "Failed to read entry while expanding data dirs");
            })
            .ok()
        })
        .filter(move |e| {
            if !e.file_type.is_dir() || e.depth == 0 {
                return false;
            }
            self::glob::match_glob_path(skyrim_data_dir, &e.path())
        })
        .map(|e| e.path())
        .collect()
}

pub async fn collect_all_fnis_injections(
    skyrim_data_dir: &str,
    fnis_entries: &PriorityMap,
) -> (Vec<OwnedFnisInjection>, Vec<Error>) {
    #[cfg(feature = "tracing")]
    tracing::debug!(
        skyrim_data_dir,
        entry_count = ALL_ENTRIES.len(),
        "Starting FNIS injection collection"
    );

    // In manual mode, you need to search everything in the `MO2/mods/` directory as if it were the meshes directory.
    // That is why `data_dirs` is defined as a Vec.
    let data_dirs = expand_data_dirs(skyrim_data_dir);

    #[cfg(feature = "tracing")]
    tracing::debug!(count = data_dirs.len(), "Expanded skyrim_data_dir");

    let mut handles = tokio::task::JoinSet::new();

    for data_dir in &data_dirs {
        for entry in ALL_ENTRIES.iter() {
            let animations_dir = data_dir
                .join("meshes")
                .join(entry.base_dir)
                .join("animations");

            #[cfg(feature = "tracing")]
            tracing::trace!(?animations_dir, "Scanning animations directory");

            let read_dir = match std::fs::read_dir(&animations_dir) {
                Ok(rd) => rd,
                Err(_e) => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(
                        ?animations_dir,
                        error = %_e,
                        "Skipping animations directory (not found or inaccessible)"
                    );
                    continue;
                }
            };

            for dir_entry in read_dir.flatten() {
                let Ok(file_type) = dir_entry.file_type() else {
                    continue;
                };
                if !file_type.is_dir() {
                    continue;
                }

                let ns_path = dir_entry.path();
                let Some(namespace) = ns_path.file_name().and_then(|n| n.to_str()) else {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(?ns_path, "Skipping namespace dir with non-UTF8 name");
                    continue;
                };
                let Some(&priority) = fnis_entries.get(namespace) else {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(namespace, "Skipping namespace not present in fnis_entries");
                    continue;
                };

                #[cfg(feature = "tracing")]
                tracing::debug!(
                    namespace,
                    priority,
                    base_dir = entry.base_dir,
                    "Spawning FNIS injection task"
                );

                let namespace = namespace.to_string();
                let entry: &'static BehaviorEntry = entry;

                handles.spawn(async move {
                    collect_fnis_injection(&ns_path, entry, &namespace, priority).await
                });
            }
        }
    }

    let mut oks = Vec::new();
    let mut errs = Vec::new();
    while let Some(result) = handles.join_next().await {
        match result {
            Ok(Ok(v)) => {
                #[cfg(feature = "tracing")]
                tracing::trace!(namespace = v.namespace, "FNIS injection collected");
                oks.push(v);
            }
            Ok(Err(e)) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %e, "FNIS injection task failed");
                errs.push(Error::from(e));
            }
            Err(join_err) => {
                #[cfg(feature = "tracing")]
                tracing::error!(error = %join_err, "FNIS injection task panicked");
                errs.push(Error::from(join_err));
            }
        }
    }

    #[cfg(feature = "tracing")]
    tracing::debug!(
        succeeded = oks.len(),
        failed = errs.len(),
        "FNIS injection collection complete"
    );

    (oks, errs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rayon::prelude::*;

    #[tokio::test]
    #[ignore = "local only"]
    async fn test_parse_relative_path() {
        let output_path = "../../dummy/debug/collect_all_fnis_injections.log";

        let fnis_entries = ["BiS_WashMe", "FNISFlyer", "FNISZoo", "XPMSE", "TKDodge"]
            .into_par_iter()
            .enumerate()
            .map(|(idx, namespace)| (namespace.to_string(), idx))
            .collect();
        let res = collect_all_fnis_injections("../../dummy/fnis_test_mods/*", &fnis_entries).await;

        std::fs::write(output_path, format!("{res:#?}")).unwrap();
    }
}
