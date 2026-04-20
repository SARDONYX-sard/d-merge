//! HKX read → convert → write pipeline.
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rayon::prelude::*;

use crate::{
    Config,
    behaviors::tasks::fnis::{
        collect::owned::OwnedFnisInjection,
        patch_gen::{alternate::group_names::AAGroupName, io_jobs::AnimIoJob},
    },
    config::OutPutTarget,
    errors::Error,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct ConversionJob {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub kind: AnimKind,
}

#[derive(Debug, Clone)]
pub(crate) enum AnimKind {
    /// Standard animation — no FNIS metadata needed.
    Standard,
    /// FNIS alternate animation — carries metadata for `aa_config.json`.
    FnisAA {
        /// e.g. `"xpe"`
        prefix: Arc<str>,
        /// e.g. `_1hmeqp`
        group_name: AAGroupName,
        /// Total slots registered by this mod for this group.
        slot_count: u64,
        /// When true, the source was under a `male/` subdirectory.
        /// The converted bytes are written to both `male/` and `female/`.
        ///
        /// # Why need this?
        /// mt_runforward.hkx did not work in OAR even when placed directly under the “animation” folder.
        ///
        /// male/mt_runforward.hkx
        /// female/mt_runforward.hkx
        ///
        /// I have no choice but to copy them to these respective locations.
        is_male_subdir: bool,
    },
}

// ---------------------------------------------------------------------------
// Pipeline entry point
// ---------------------------------------------------------------------------

/// Runs the full HKX read → convert → write pipeline in parallel.
#[must_use]
pub(crate) fn run(jobs: Vec<ConversionJob>, output_target: OutPutTarget) -> Vec<Error> {
    // Stage 1: read
    let read_results: Vec<Result<ConversionBytes, Error>> = jobs
        .into_par_iter()
        .filter_map(|job| read(job, output_target))
        .collect();

    // Stage 2: convert (in-memory)
    let (converted, mut errors): (Vec<ConversionBytes>, Vec<Error>) =
        read_results.into_par_iter().partition_map(|res| match res {
            Ok(mut b) => match convert(&b.input_path, &mut b.bytes, output_target) {
                Ok(()) => rayon::iter::Either::Left(b),
                Err(e) => rayon::iter::Either::Right(e),
            },
            Err(e) => rayon::iter::Either::Right(e),
        });

    // Stage 3: write
    errors.par_extend(
        converted
            .into_par_iter()
            .filter_map(|b| super::write_file(&b.output_path, &b.bytes).err()),
    );

    errors
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

struct ConversionBytes {
    input_path: PathBuf,
    output_path: PathBuf,
    bytes: Vec<u8>,
}

fn read(job: ConversionJob, output_target: OutPutTarget) -> Option<Result<ConversionBytes, Error>> {
    use std::{borrow::Cow, fs::File, io::Read, path::Path};

    let actual_input: Cow<Path> = if !job.input_path.exists() {
        if matches!(job.kind, AnimKind::FnisAA { .. }) {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                path = %job.input_path.display(),
                "Input file does not exist; skipping (FNIS AltAnim → OAR)."
            );
            return None;
        } else if let Some(found) = find_case_insensitive(&job.input_path) {
            Cow::Owned(found)
        } else {
            return Some(Err(Error::FNISHkxIoError {
                path: job.input_path.clone(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
            }));
        }
    } else {
        Cow::Borrowed(&job.input_path)
    };

    // --- Step 1: Read only 17 bytes (header) ---
    let mut file = match File::open(actual_input.as_ref()) {
        Ok(f) => f,
        Err(e) => {
            return Some(Err(Error::FNISHkxIoError {
                path: actual_input.into_owned(),
                source: e,
            }));
        }
    };

    let mut header = [0_u8; 17];
    if let Err(e) = file.read_exact(&mut header) {
        return Some(Err(Error::FNISHkxIoError {
            path: actual_input.into_owned(),
            source: e,
        }));
    }

    let current_format = match check_header(header, &actual_input, output_target) {
        Ok(f) => f,
        Err(e) => return Some(Err(e)),
    };

    if matches!(job.kind, AnimKind::Standard) && current_format == output_target {
        return None; // If no conversion needed -> skip without full read
    }

    // --- Step 2: Read full file only if needed ---
    let bytes = match std::fs::read(actual_input.as_ref()) {
        Ok(b) => b,
        Err(e) => {
            return Some(Err(Error::FNISHkxIoError {
                path: actual_input.into_owned(),
                source: e,
            }));
        }
    };

    Some(Ok(ConversionBytes {
        input_path: actual_input.into_owned(),
        output_path: job.output_path,
        bytes,
    }))
}

fn convert(
    input_path: &Path,
    bytes: &mut Vec<u8>,
    output_target: OutPutTarget,
) -> Result<(), Error> {
    use serde_hkx::bytes::serde::hkx_header::HkxHeader;
    use serde_hkx_features::ClassMap;

    if bytes.is_empty() {
        return Ok(());
    }

    let class_map: ClassMap = serde_hkx::from_bytes(bytes).map_err(|e| Error::HkxDeError {
        path: input_path.to_path_buf(),
        source: e,
    })?;

    let header = match output_target {
        OutPutTarget::SkyrimLe => HkxHeader::new_skyrim_le(),
        OutPutTarget::SkyrimSe => HkxHeader::new_skyrim_se(),
    };

    *bytes = serde_hkx::to_bytes(&class_map, &header).map_err(|e| Error::HkxSerError {
        path: input_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}

fn check_header(
    header: [u8; 17],
    input_path: &Path,
    output_format: OutPutTarget,
) -> Result<OutPutTarget, Error> {
    const TAG_MAGIC: [u8; 8] = [0x1E, 0x0D, 0xB0, 0xCA, 0xCE, 0xFA, 0x11, 0xD0];
    if header[0..8] == TAG_MAGIC {
        #[cfg(feature = "tracing")]
        tracing::info!(path = %input_path.display(), "Tag file detected; Skipped.");
        return Ok(output_format);
    }

    const TAG_XML_START: &[u8; 16] = b"<?xml version=\"1";
    if &header[0..16] == TAG_XML_START {
        #[cfg(feature = "tracing")]
        tracing::info!(path = %input_path.display(), "XML Tag file detected; Skipped.");
        return Ok(output_format);
    }

    const HKX_MAGIC: [u8; 8] = [0x57, 0xE0, 0xE0, 0x57, 0x10, 0xC0, 0xC0, 0x10];
    if header[0..8] != HKX_MAGIC {
        #[cfg(feature = "tracing")]
        {
            let err = Error::FNISHkxInvalidMagic {
                input_path: input_path.to_path_buf(),
                target: output_format,
                magic_bytes: header,
            };
            tracing::warn!(path = %input_path.display(), "Skipped.: {err}");
        }
        return Ok(output_format);
    }

    match header[16] {
        4 => Ok(OutPutTarget::SkyrimLe),
        8 => Ok(OutPutTarget::SkyrimSe),
        n => Err(Error::FNISHkxInvalidHeader {
            input_path: input_path.to_path_buf(),
            target: output_format,
            actual: n,
        }),
    }
}

/// Case-insensitive file lookup for Unix systems.
fn find_case_insensitive(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let file_name = path.file_name()?;
    std::fs::read_dir(parent).ok()?.find_map(|e| {
        let e = e.ok()?;
        e.file_name()
            .eq_ignore_ascii_case(file_name)
            .then(|| e.path())
    })
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Prepare a list of conversion jobs from animation filenames and `OwnedFnisInjection` metadata.
///
/// This stage **performs no IO** and only calculates paths. The final job list can be
/// safely passed to `run_conversion_jobs`.
///
/// The behavior file path is appended as the last job.
///
/// # Returns
/// Vector of `ConversionJob` ready for parallel processing.
#[must_use]
pub(crate) fn prepare_conversion_jobs(
    animations: &[&str],
    owned_data: &OwnedFnisInjection,
    config: &Config,
) -> Vec<AnimIoJob> {
    let base_dir = &owned_data.behavior_entry.base_dir;
    let namespace = &owned_data.namespace;
    let output_dir = &config.output_dir;

    animations
        .par_iter()
        .map(|anim| {
            let anim_file = anim.replace("\\", "/"); // normalize path separators
            let input_path = owned_data.animations_mod_dir.join(&anim_file);
            let mut output_path = output_dir.join("meshes");
            output_path.push(base_dir);
            output_path.push(namespace);
            output_path.push(&anim_file);

            AnimIoJob::Hkx(ConversionJob {
                input_path,
                output_path,
                kind: AnimKind::Standard,
            })
        })
        .collect()
}

/// Prepare a single conversion job for the behavior file itself.
#[must_use]
pub(crate) fn prepare_behavior_conversion_job(
    owned_data: &OwnedFnisInjection,
    config: &Config,
) -> Option<AnimIoJob> {
    match owned_data.to_behavior_path() {
        Ok((input_path, output_inner)) => {
            let output_path = config.output_dir.join(output_inner);
            Some(AnimIoJob::Hkx(ConversionJob {
                input_path,
                output_path,
                kind: AnimKind::Standard,
            }))
        }
        Err(_err) => {
            #[cfg(feature = "tracing")]
            tracing::error!(
                "Failed to prepare behavior file conversion job(Since it should be checked at the collect::owned stage, it should not normally result in an error.): {_err}"
            );
            None
        }
    }
}
