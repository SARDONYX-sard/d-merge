use crate::{
    behaviors::tasks::fnis::collect::{owned::collect_fnis_injection, types::OwnedActorFnisMap},
    PriorityMap,
};

use rayon::prelude::*;
use std::path::Path;

/// Collect FNIS injections for all actors under `meshes/actors/*/animations/<namespace>/`,
/// including `Character/_1stperson/animations/<namespace>/`.
///
/// - `fnis_entries` = list of target FNIS namespaces to search for.
/// - `skyrim_data_dir` = Skyrim `Data` directory.
/// - Returns `OwnedActorFnisMap` keyed by actor name.
///   - Key examples: `"Character"`, `"Character/_1stperson"`, `"cow"`
///   - Value: Vec of `OwnedFnisInjection` per namespace found
pub fn collect_all_fnis_injections(
    skyrim_data_dir: &Path,
    fnis_entries: PriorityMap,
) -> OwnedActorFnisMap {
    let cloned_fnis_entries = fnis_entries.clone();
    let results = OwnedActorFnisMap::new();

    let actors_dir = {
        let mut actors_dir = skyrim_data_dir.join("meshes");
        actors_dir.push("actors");
        actors_dir
    };

    let walk_dir = jwalk::WalkDirGeneric::<(usize, bool)>::new(&actors_dir)
        .skip_hidden(true)
        .process_read_dir(move |depth, path, _read_dir_state, children| {
            // depth=2: animations/<namespace>/...
            children.par_iter_mut().flatten().for_each(|child| {
                let name = child.file_name.to_string_lossy();

                // Special case: Character's _1stperson is searchable
                if depth == Some(1) && name == "Character" {
                    return;
                }

                // depth=2: Directly under animations/<namespace>/...
                if depth == Some(2) && !cloned_fnis_entries.contains_key(name.as_ref()) {
                    child.read_children_path = None; // Skip if not contained in namespace
                }

                // depth=2: Skip subdirectories other than Character's _1stperson
                if depth == Some(2) && path.ends_with("Character") && name != "_1stperson" {
                    child.read_children_path = None;
                }

                // depth >=2: Skip unnecessary subdirectories under other actors
                if depth >= Some(2) && !path.ends_with("Character") {
                    child.read_children_path = None;
                }
            });
        });

    walk_dir
        .into_iter()
        .par_bridge()
        .filter_map(|entry| entry.ok())
        .for_each(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return;
            }
            let Some(ns_name) = path.file_name()else {
                return; // I think unreachable.
            };
            let ns_name = ns_name.to_string_lossy();

            // actor(path = .../actors/<actor>/animations/<namespace>)
            let Some(actor_name) = path.ancestors().nth(2)
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string()) else {
                    tracing::warn!("Failed to compute actor_name for path: {}", path.display());
                    return;
                };

            let Some(priority) = fnis_entries.get(ns_name.as_ref()).copied() else {
                tracing::info!("This mod is skipped because it is not included in the enabled fnis_entries.: {ns_name}");
                return;
            } ;

            match collect_fnis_injection(&path, priority) {
                Ok(injection) => {
                    results.inner().entry(actor_name).or_default().push(injection);
                }
                Err(e) => {
                    tracing::warn!(%actor_name, namespace = %ns_name, error=%e, "Failed to collect FNIS injection");
                }
            }
        });

    results
}
