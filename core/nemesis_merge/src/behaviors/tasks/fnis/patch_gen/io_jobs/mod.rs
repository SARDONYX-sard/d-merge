//! Dispatches animation I/O jobs across three parallel branches:
//! HKX conversion, static config writes, and FNIS AA deferred config writes.
pub mod config;
pub mod fnis_aa;
pub mod hkx;

use std::sync::Arc;

use rayon::prelude::*;

pub use self::hkx::{AnimKind, ConversionJob};
use crate::{
    behaviors::tasks::fnis::patch_gen::alternate::{FnisAANamespaceConfigJob, FnisAASlotConfigJob},
    config::OutPutTarget,
    errors::Error,
};

#[derive(Debug)]
pub enum AnimIoJob {
    Hkx(ConversionJob),
    /// Static namespace-level OAR config (no base dependency).
    FnisAANamespaceConfig(FnisAANamespaceConfigJob),
    /// Per-slot OAR config whose `Value B` depends on the computed base.
    FnisAASlotConfig(FnisAASlotConfigJob),
}

/// Run the HKX conversion jobs in parallel.
///
/// Checks each file header first; skips files already matching the target format.
/// Returns all errors encountered during conversion.
///
/// # Note
/// AAConfig is pre-built by the caller; None when no FnisAA jobs exist.
///
/// # Returns Errors
/// Returns a collection of errors if any file:
/// - Cannot be read (I/O errors),
/// - Has invalid HKX magic numbers,
/// - Has a pointer size that cannot be determined.
#[must_use]
pub fn run_conversion_jobs(
    jobs: Vec<AnimIoJob>,
    output_target: OutPutTarget,
    aa_base_map: Option<&super::alternate::aa_config::BaseMap>,
) -> Vec<Error> {
    #[cfg(feature = "tracing")]
    tracing::debug!("jobs to run: {:#?}", jobs);

    let (hkx_jobs, namespace_config_jobs, slot_config_jobs) = partition_jobs(jobs);

    let mut hkx_errors = vec![];
    let mut namespace_config_errors = vec![];
    let mut slot_config_errors = vec![];

    rayon::scope(|s| {
        s.spawn(|_| {
            hkx_errors = hkx::run(hkx_jobs, output_target);
        });
        s.spawn(|_| {
            namespace_config_errors = config::run(namespace_config_jobs);
        });
        s.spawn(|_| {
            slot_config_errors = fnis_aa::run(slot_config_jobs, aa_base_map);
        });
    });

    let mut errors = hkx_errors;
    errors.extend(namespace_config_errors);
    errors.extend(slot_config_errors);
    errors
}

/// Partitions jobs into three buckets in a single parallel pass.
fn partition_jobs(
    jobs: Vec<AnimIoJob>,
) -> (
    Vec<ConversionJob>,
    Vec<FnisAANamespaceConfigJob>,
    Vec<FnisAASlotConfigJob>,
) {
    jobs.into_par_iter()
        .fold(
            || (Vec::new(), Vec::new(), Vec::new()),
            |mut acc, job| {
                match job {
                    AnimIoJob::Hkx(j) => {
                        // Mirror to female/ before moving j into acc
                        if let Some(female_job) = mirror_male_to_female(&j) {
                            acc.0.push(female_job);
                        }

                        acc.0.push(j);
                    }
                    AnimIoJob::FnisAANamespaceConfig(j) => acc.1.push(j),
                    AnimIoJob::FnisAASlotConfig(j) => acc.2.push(j),
                }
                acc
            },
        )
        .reduce(
            || (Vec::new(), Vec::new(), Vec::new()),
            |mut a, b| {
                a.0.extend(b.0);
                a.1.extend(b.1);
                a.2.extend(b.2);
                a
            },
        )
}

pub(super) fn write_file(output_path: &std::path::Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::FNISHkxIoError {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    std::fs::write(output_path, bytes).map_err(|e| Error::FNISHkxIoError {
        path: output_path.into(),
        source: e,
    })
}

/// Creates a mirrored `female/` copy of a `male/` HKX job.
///
/// Returns `None` when `job.kind` is not `FnisAA { is_male_subdir: true, .. }`.
fn mirror_male_to_female(job: &ConversionJob) -> Option<ConversionJob> {
    let AnimKind::FnisAA {
        is_male_subdir: true,
        prefix,
        group_name,
        slot_count,
    } = &job.kind
    else {
        return None;
    };

    let Some(grandparent) = job.output_path.parent().and_then(|p| p.parent()) else {
        #[cfg(feature = "tracing")]
        tracing::error!(
            path = %job.output_path.display(),
            "Failed to get grandparent of male animation path; Skip to avoid panic."
        );
        return None;
    };

    let Some(file_name) = job.output_path.file_name() else {
        #[cfg(feature = "tracing")]
        tracing::error!(
            path = %job.output_path.display(),
            "Failed to get file name of male animation path; Skip to avoid panic."
        );
        return None;
    };

    let female_output = grandparent.join("female").join(file_name);

    Some(ConversionJob {
        input_path: job.input_path.clone(),
        output_path: female_output,
        kind: AnimKind::FnisAA {
            prefix: Arc::clone(prefix),
            group_name: *group_name,
            slot_count: *slot_count,
            is_male_subdir: false, // prevent infinite mirroring
        },
    })
}
