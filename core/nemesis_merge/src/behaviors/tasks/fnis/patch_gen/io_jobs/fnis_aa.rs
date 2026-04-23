//! Per-slot OAR config writes with deferred base resolution.
use std::sync::Arc;

use rayon::prelude::*;

use crate::{
    behaviors::tasks::fnis::patch_gen::alternate::{
        FnisAASlotConfigJob, aa_config::BaseMap, oar_json::new_fnis_aa_slot_config_json,
    },
    errors::Error,
};

/// Resolves each slot's base and writes its `config.json` in parallel.
#[must_use]
pub(crate) fn run(jobs: Vec<FnisAASlotConfigJob>, base_map: Option<&BaseMap>) -> Vec<Error> {
    jobs.into_par_iter()
        .enumerate()
        .filter_map(|(i, job)| {
            let base = base_map
                .and_then(|m| m.get(&(Arc::clone(&job.prefix), job.group_name.group_id())))
                .copied()
                .unwrap_or(1); // fallback: base=1 (should not happen if caller built map correctly)

            // NOTE:
            // We assign a unique fallback priority to avoid collisions between configs.
            //
            // This is not strictly required for functionality. However, in a previous
            // Pandora implementation where FNIS AA to OAR configs shared the same
            // priority, users reported issues due to OAR emitting warnings for
            // duplicate priorities. To avoid those warnings, we ensure uniqueness here.
            let fallback_priority = i32::MAX - (i as i32);

            let config = new_fnis_aa_slot_config_json(
                &job.group_config_dir,
                job.group_name,
                job.slot,
                base,
                fallback_priority,
                job.slot_config.as_deref(),
            );
            super::write_file(&job.output_path, config.as_bytes()).err()
        })
        .collect()
}
