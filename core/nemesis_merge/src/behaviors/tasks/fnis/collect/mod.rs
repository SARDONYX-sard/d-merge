pub mod owned;

use crate::behaviors::tasks::fnis::collect::owned::{collect_fnis_injection, OwnedFnisInjection};
use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::{
    BehaviorEntry, AUXBONES, CREATURES, HUMANOID, PLANTS_ACTIVATORS, SKELETONS,
};
use crate::errors::Error;
use crate::PriorityMap;
use std::path::PathBuf;

/// One job describing a single FNIS injection task.
/// No IO is performed inside this struct.
/// IO is only performed later inside the parallel stage.
struct FnisJob {
    ns_path: PathBuf,
    entry: &'static BehaviorEntry,
    namespace: String,
    priority: usize,
}

/// Collects all FNIS injection jobs by scanning the filesystem in a
/// single-threaded manner.
///
/// This phase avoids nested parallel IO entirely, which prevents
/// USVFS (RecursiveBenaphore) deadlocks.
///
/// The returned Vec<FnisJob> contains all necessary metadata but
/// does not perform any heavy file operations.
fn gather_fnis_jobs(skyrim_data_dir: &str, fnis_entries: &PriorityMap) -> Vec<FnisJob> {
    let mut jobs = Vec::new();

    let all_entries = HUMANOID
        .values()
        .chain(CREATURES.values())
        // Flag to ensure "draugr" from CREATURES is only processed once
        .chain(SKELETONS.values().filter(|e| e.behavior_object != "draugr"))
        .chain(AUXBONES.values())
        .chain(PLANTS_ACTIVATORS.values());

    for entry in all_entries {
        let glob_pattern = format!("{skyrim_data_dir}/meshes/{}/animations/*", entry.base_dir);

        // glob may trigger some IO, but this phase is single-threaded, so USVFS-safe
        if let Ok(paths) = collect_paths(&glob_pattern) {
            for ns_path in paths {
                if !ns_path.is_dir() {
                    continue;
                }

                let namespace = match ns_path.file_name() {
                    Some(n) => n.to_string_lossy().to_string(),
                    None => continue,
                };

                let priority = match fnis_entries.get(namespace.as_str()) {
                    Some(p) => *p,
                    None => continue,
                };

                jobs.push(FnisJob {
                    ns_path,
                    entry,
                    namespace,
                    priority,
                });
            }
        }
    }

    jobs
}

/// Processes FNIS jobs in parallel.
/// This is the ONLY parallel stage, ensuring USVFS does not see nested IO.
///
/// Heavy file IO (reading FNIS_*_List.txt, etc.) is performed here,
/// but since there is only one par_iter, USVFS behaves correctly.
async fn process_fnis_jobs_parallel(jobs: Vec<FnisJob>) -> (Vec<OwnedFnisInjection>, Vec<Error>) {
    let mut handles = tokio::task::JoinSet::new();
    for job in jobs {
        handles.spawn(async move {
            Ok(
                collect_fnis_injection(&job.ns_path, job.entry, &job.namespace, job.priority)
                    .await?,
            )
        });
    }

    let mut oks = Vec::new();
    let mut errs = Vec::new();
    while let Some(result) = handles.join_next().await {
        match result {
            Err(join_err) => {
                errs.push(Error::from(join_err));
            }
            Ok(inner) => match inner {
                Ok(v) => oks.push(v),
                Err(e) => errs.push(e),
            },
        }
    }

    (oks, errs)
}

/// Public API: collect all FNIS injections.
/// Wraps the job-gathering and parallel processing stages.
///
/// # Why this design?
/// - All directory scanning is done single-threaded
/// - All IO-heavy tasks run in one parallel stage
/// - Avoids nested parallel IO, which breaks USVFS (RecursiveBenaphore)
pub async fn collect_all_fnis_injections(
    skyrim_data_dir: &str,
    fnis_entries: &PriorityMap,
) -> (Vec<OwnedFnisInjection>, Vec<Error>) {
    let jobs = gather_fnis_jobs(skyrim_data_dir, fnis_entries);
    process_fnis_jobs_parallel(jobs).await
}

/// Collect case-insensitive paths using glob.
/// Safe to call from the single-threaded stage.
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
