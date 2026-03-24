//! Static namespace-level OAR config writes (no base dependency).
use rayon::prelude::*;

use crate::{
    behaviors::tasks::fnis::patch_gen::alternate::FnisAANamespaceConfigJob, errors::Error,
};

/// Writes all static namespace `config.json` files in parallel.
#[must_use]
pub fn run(jobs: Vec<FnisAANamespaceConfigJob>) -> Vec<Error> {
    jobs.into_par_iter()
        .filter_map(|job| super::write_file(&job.output_path, job.config.as_bytes()).err())
        .collect()
}
