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

    /// Do we need to copy to output_path even without conversion?
    ///
    /// This is required for FNIS AltAnim to OAR and should be set to true.
    pub need_copy: bool,
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
        .iter()
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
                need_copy: false,
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
                need_copy: false,
            }))
        }
        Err(_err) => {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to prepare behavior file conversion job(Since it should be checked at the collect::owned stage, it should not normally result in an error.): {_err}");
            None
        }
    }
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
    // TODO: Avoid using `rayon::par_iter` because it causes errors due to mutexes in MO2
    jobs.iter()
        .filter_map(|job| match job {
            AnimIoJob::Hkx(conversion_job) => convert_hkx(
                &conversion_job.input_path,
                &conversion_job.output_path,
                output_target,
                conversion_job.need_copy,
            )
            .err(),
            AnimIoJob::Config(alt_anim_config_job) => write_file(
                &alt_anim_config_job.output_path,
                alt_anim_config_job.config.as_bytes(),
                output_target,
            )
            .err(),
        })
        .collect()
}

/// Check the HKX file header to determine its format.
///
/// # Note
/// use IO operations internally; avoid calling this function within `rayon::par_iter` directly.
fn check_hkx_header(
    input_path: &Path,
    output_format: crate::OutPutTarget,
) -> Result<crate::OutPutTarget, Error> {
    let header = match std::fs::File::open(input_path).and_then(|mut f| {
        use std::io::Read;
        let mut buf = [0_u8; 17];
        f.read_exact(&mut buf)?;
        Ok(buf)
    }) {
        Ok(header) => header,
        Err(e) => {
            return Err(Error::FNISHkxIoError {
                path: input_path.to_path_buf(),
                target: output_format,
                source: e,
            })
        }
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

fn convert_hkx(
    input_path: &Path,
    output_path: &Path,
    output_format: crate::OutPutTarget,
    need_copy: bool,
) -> Result<(), Error> {
    use serde_hkx::bytes::serde::hkx_header::HkxHeader;
    use serde_hkx_features::ClassMap;
    use std::borrow::Cow;

    // NOTE: Exists sometimes misjudges virtualization as unstable for `rayon::par_iter` in MO2.
    let actual_input: Cow<Path> = if input_path.exists() {
        Cow::Borrowed(input_path)
    } else if let Some(found) = find_case_insensitive(input_path) {
        Cow::Owned(found)
    } else {
        #[cfg(feature = "tracing")]
        tracing::info!(
            "FNIS alternative animation input_path file '{}' not found. Then Skipped.",
            input_path.display()
        );
        return Ok(());
    };

    let current_format = check_hkx_header(&actual_input, output_format)?;
    if current_format == output_format {
        if need_copy {
            std::fs::copy(&actual_input, output_path).map_err(|e| Error::FNISHkxIoError {
                path: actual_input.to_path_buf(),
                target: output_format,
                source: e,
            })?;
        }
        return Ok(());
    }

    // start conversion code ---

    let bytes = std::fs::read(&actual_input).map_err(|e| Error::FNISHkxIoError {
        path: actual_input.to_path_buf(),
        target: output_format,
        source: e,
    })?;

    let class_map: ClassMap = serde_hkx::from_bytes(&bytes).map_err(|e| Error::HkxDeError {
        path: actual_input.to_path_buf(),
        source: e,
    })?;

    let header = match output_format {
        crate::OutPutTarget::SkyrimLe => HkxHeader::new_skyrim_le(),
        crate::OutPutTarget::SkyrimSe => HkxHeader::new_skyrim_se(),
    };

    let bytes = serde_hkx::to_bytes(&class_map, &header).map_err(|e| Error::HkxSerError {
        path: actual_input.to_path_buf(),
        source: e,
    })?;

    write_file(output_path, &bytes, output_format)
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
