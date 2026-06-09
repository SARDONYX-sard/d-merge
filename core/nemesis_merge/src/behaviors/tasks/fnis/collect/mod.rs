pub(crate) mod owned;

use crate::{
    PriorityMap,
    behaviors::{
        priority_ids::parse_fnis_list_path,
        tasks::fnis::{
            collect::owned::{OwnedFnisInjection, collect_fnis_injection},
            patch_gen::generated_behaviors::{
                AUXBONES, CREATURES, HUMANOID, PLANTS_ACTIVATORS, SKELETONS,
            },
        },
    },
    errors::Error,
};

/// Collect all [`OwnedFnisInjection`]s from the given FNIS priority map.
///
/// Each entry in `fnis_entries` is a `(list_path, priority)` pair where
/// `list_path` is the absolute path to a `FNIS_*_List.txt` file.
/// The path is parsed to extract the [`BehaviorEntry`] and namespace,
/// then [`collect_fnis_injection`] is spawned as a Tokio task per entry.
///
/// # Returns
/// A tuple of `(succeeded, failed)` — errors are collected rather than
/// propagated so that a single bad entry does not abort the whole collection.
pub(crate) async fn collect_all_fnis_injections(
    fnis_entries: &PriorityMap,
) -> (Vec<OwnedFnisInjection>, Vec<Error>) {
    #[cfg(feature = "tracing")]
    tracing::debug!(entry_count = fnis_entries.len(), "Starting FNIS injection collection");

    let mut handles = tokio::task::JoinSet::new();

    for (list_path, &priority) in fnis_entries {
        let Some(parsed) = parse_fnis_list_path(list_path) else {
            #[cfg(feature = "tracing")]
            tracing::warn!(list_path = list_path, "Failed to parse FNIS list path");
            continue;
        };

        let Some(entry) = (match parsed.behavior_object {
            Some(key) => CREATURES
                .get(key)
                .or_else(|| SKELETONS.get(key))
                .or_else(|| AUXBONES.get(key))
                .or_else(|| PLANTS_ACTIVATORS.get(key)),
            None => match parsed.is_1st_person {
                true => HUMANOID.get("character/_1stperson"),
                false => HUMANOID.get("character"),
            },
        }) else {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                behavior_object = parsed.behavior_object,
                is_1st_person = parsed.is_1st_person,
                "No BehaviorEntry matched for FNIS list path"
            );
            continue;
        };

        let list_path = std::path::Path::new(list_path);
        let Some(animations_mod_dir) = list_path.parent().map(|p| p.to_path_buf()) else {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Failed to get parent dir of FNIS list path(path = {})",
                list_path.display()
            );
            continue;
        };

        let namespace = parsed.namespace.to_string();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            namespace,
            priority,
            base_dir = entry.base_dir,
            "Spawning FNIS injection task"
        );

        let list_path = list_path.to_path_buf();
        handles.spawn(async move {
            collect_fnis_injection(&animations_mod_dir, entry, &namespace, priority, list_path)
                .await
        });
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
    use crate::tests::patches_builder::{PatchMapsConfig, build_patch_maps};

    #[tokio::test]
    #[ignore = "local only"]
    async fn test_parse_relative_path() {
        let _guard = quick_tracing::builder::LoggerBuilder::default()
            .filter(tracing::level_filters::LevelFilter::WARN)
            .build();
        let output_path = "../../dummy/debug/collect_all_fnis_injections.log";

        let patches = build_patch_maps(PatchMapsConfig {
            pattern: "../../dummy/fnis_test_mods/*",
            ..Default::default()
        });
        let res = collect_all_fnis_injections(&patches.fnis_entries).await;

        std::fs::write(output_path, format!("{res:#?}")).unwrap();
    }
}
