//! Per-slot OAR config writes with deferred base resolution.
use std::sync::Arc;

use rayon::prelude::*;

use crate::{
    behaviors::tasks::fnis::patch_gen::alternate::{
        aa_config::BaseMap, oar_json::new_fnis_aa_slot_config_json, FnisAASlotConfigJob,
    },
    errors::Error,
};

/// Resolves each slot's base and writes its `config.json` in parallel.
#[must_use]
pub fn run(jobs: Vec<FnisAASlotConfigJob>, base_map: Option<&BaseMap>) -> Vec<Error> {
    jobs.into_par_iter()
        .filter_map(|job| {
            let base = base_map
                .and_then(|m| m.get(&(Arc::clone(&job.prefix), job.group_name.group_id())))
                .copied()
                .unwrap_or(1); // fallback: base=1 (should not happen if caller built map correctly)

            let config = new_fnis_aa_slot_config_json(
                &job.group_config_dir,
                job.group_name,
                job.slot,
                base,
                job.slot_config.as_deref(),
            );
            super::write_file(&job.output_path, config.as_bytes()).err()
        })
        .collect()
}
