use rayon::prelude::*;
use std::path::{Path, PathBuf};

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::patch_gen::alternative::AltAnimConfigJob;
use crate::config::{Config, OutPutTarget};
use crate::errors::Error;

#[derive(Debug)]
pub enum AnimIoJob {
    Hkx(ConversionJob),
    Config(AltAnimConfigJob),
}

/// A single HKX conversion job, containing input & output paths.
#[derive(Debug)]
pub struct ConversionJob {
    /// Path to the source FNIS HKX animation file
    pub input_path: PathBuf,
    /// Path to the target converted HKX file
    pub output_path: PathBuf,

    /// Is this job optional? If true, missing input files will be skipped without error.
    ///
    /// This is required for FNIS AltAnim to OAR and should be set to true.
    pub is_optional: bool,
}

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
pub fn prepare_conversion_jobs(
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
                is_optional: false,
            })
        })
        .collect()
}

/// Prepare a single conversion job for the behavior file itself.
#[must_use]
pub fn prepare_behavior_conversion_job(
    owned_data: &OwnedFnisInjection,
    config: &Config,
) -> Option<AnimIoJob> {
    match owned_data.to_behavior_path() {
        Ok((input_path, output_inner)) => {
            let output_path = config.output_dir.join(output_inner);
            Some(AnimIoJob::Hkx(ConversionJob {
                input_path,
                output_path,
                is_optional: false,
            }))
        }
        Err(_err) => {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to prepare behavior file conversion job(Since it should be checked at the collect::owned stage, it should not normally result in an error.): {_err}");
            None
        }
    }
}

/// Check the HKX file header to determine its format.
///
/// # Note
/// use IO operations internally; avoid calling this function within `rayon::par_iter` directly.
fn check_hkx_header(
    bytes: &[u8],
    input_path: &Path,
    output_format: crate::OutPutTarget,
) -> Result<crate::OutPutTarget, Error> {
    if bytes.len() < 17 {
        return Err(Error::FNISHkxInvalidHeader {
            input_path: PathBuf::new(),
            target: output_format,
            actual: bytes.len() as u8,
        });
    }

    let header = {
        let mut header = [0_u8; 17];
        header.copy_from_slice(&bytes[0..17]);
        header
    };

    // Actually, both LE and SE versions of hkt can be loaded, and there are mods disguised as hkx files. Example: Ride Sharing's `rsh_horsepinion.hkx`
    // This is the processing for that.
    // NOTE: Tag files cannot be converted by serde_hkx, so they are skipped.
    let is_tag_file = {
        /// Tag file(.hkt) magic bytes
        const EXPECTED_MAGIC: [u8; 8] = [
            0x1E, 0x0D, 0xB0, 0xCA, // magic0
            0xCE, 0xFA, 0x11, 0xD0, // magic1
        ];
        header[0..8] == EXPECTED_MAGIC
    };
    if is_tag_file {
        #[cfg(feature = "tracing")]
        tracing::info!(
            path = %input_path.display(),
            "Tag files cannot be converted by serde_hkx, so they are skipped."
        );
        return Ok(output_format);
    }

    let is_hkx = {
        /// .hkx magic bytes
        const EXPECTED_MAGIC: [u8; 8] = [
            0x57, 0xE0, 0xE0, 0x57, // magic0
            0x10, 0xC0, 0xC0, 0x10, // magic1
        ];
        header[0..8] == EXPECTED_MAGIC
    };
    if !is_hkx {
        return Err(Error::FNISHkxInvalidMagic {
            input_path: input_path.to_path_buf(),
            target: output_format,
            magic_bytes: header,
        });
    }

    // check ptr size
    let ptr_size = header[16];
    let current_format = match ptr_size {
        4 => crate::OutPutTarget::SkyrimLe,
        8 => crate::OutPutTarget::SkyrimSe,
        _ => {
            return Err(Error::FNISHkxInvalidHeader {
                input_path: input_path.to_path_buf(),
                target: output_format,
                actual: ptr_size,
            })
        }
    };

    Ok(current_format)
}

/// This is necessary because Unix systems are case-sensitive.
fn find_case_insensitive(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let file_name = path.file_name()?;

    for entry in std::fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        if name.eq_ignore_ascii_case(file_name) {
            return Some(entry.path());
        }
    }
    None
}

fn write_file(output_path: &Path, bytes: &[u8], output_format: OutPutTarget) -> Result<(), Error> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::FNISHkxIoError {
            path: parent.to_path_buf(),
            target: output_format,
            source: e,
        })?;
    }

    std::fs::write(output_path, bytes).map_err(|e| Error::FNISHkxIoError {
        path: output_path.into(),
        target: output_format,
        source: e,
    })?;
    Ok(())
}

struct ConversionBytes {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub bytes: Vec<u8>,
}

/// Run the HKX conversion jobs in parallel.
///
/// Checks each file header first; skips files already matching the target format.
/// Returns all errors encountered during conversion.
///
/// # Returns Errors
/// Returns a collection of errors if any file:
/// - Cannot be read (I/O errors),
/// - Has invalid HKX magic numbers,
/// - Has a pointer size that cannot be determined.
#[must_use]
pub fn run_conversion_jobs(jobs: Vec<AnimIoJob>, output_target: OutPutTarget) -> Vec<Error> {
    // separate hkx / cofig
    let (hkx_jobs, config_jobs): (Vec<_>, Vec<_>) =
        jobs.into_par_iter().partition_map(|job| match job {
            AnimIoJob::Hkx(hkx_job) => rayon::iter::Either::Left(hkx_job),
            AnimIoJob::Config(cfg_job) => rayon::iter::Either::Right(cfg_job),
        });

    // --- Stage 1: Read all files in parallel ---
    let read_results: Vec<Result<ConversionBytes, Error>> = hkx_jobs
        .into_par_iter()
        .filter_map(|job| read_hkx_bytes(job, output_target))
        .collect();

    // --- Stage 2: Convert all in-memory ---
    let (converted_results, mut errors): (Vec<ConversionBytes>, Vec<Error>) =
        read_results.into_par_iter().partition_map(|res| match res {
            Ok(mut bytes_struct) => match convert_hkx_bytes(
                &bytes_struct.input_path,
                &mut bytes_struct.bytes,
                output_target,
            ) {
                Ok(()) => rayon::iter::Either::Left(bytes_struct),
                Err(e) => rayon::iter::Either::Right(e),
            },
            Err(e) => rayon::iter::Either::Right(e),
        });

    // --- Stage 3: Write all files in parallel ---
    errors.par_extend(config_jobs.into_par_iter().filter_map(|res| {
        write_file(&res.output_path, res.config.as_bytes(), output_target).err()
    }));
    errors.par_extend(
        converted_results
            .into_par_iter()
            .filter_map(|res| write_file(&res.output_path, &res.bytes, output_target).err()),
    );

    errors
}

fn read_hkx_bytes(
    job: ConversionJob,
    output_target: OutPutTarget,
) -> Option<Result<ConversionBytes, Error>> {
    use std::borrow::Cow;

    let actual_input = if job.input_path.exists() {
        Cow::Borrowed(&job.input_path)
    } else if let Some(found) = find_case_insensitive(&job.input_path) {
        Cow::Owned(found)
    } else if job.is_optional {
        return None;
    } else {
        return Some(Err(Error::FNISHkxIoError {
            path: job.input_path.clone(),
            target: output_target,
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
        }));
    };

    let bytes = match std::fs::read(actual_input.as_ref()) {
        Ok(b) => b,
        Err(e) => {
            return Some(Err(Error::FNISHkxIoError {
                path: actual_input.to_path_buf(),
                target: output_target,
                source: e,
            }));
        }
    };

    Some(Ok(ConversionBytes {
        input_path: actual_input.into_owned(),
        output_path: job.output_path.clone(),
        bytes,
    }))
}

fn convert_hkx_bytes(
    input_path: &Path,
    bytes: &mut Vec<u8>,
    output_target: OutPutTarget,
) -> Result<(), Error> {
    use serde_hkx::bytes::serde::hkx_header::HkxHeader;
    use serde_hkx_features::ClassMap;

    if bytes.is_empty() {
        // Optional skipped file
        return Ok(());
    }

    let current_format = check_hkx_header(bytes, input_path, output_target)?;
    if current_format == output_target {
        return Ok(()); // no conversion needed
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
