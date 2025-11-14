use std::path::Path;

use rayon::iter::Either;
use rayon::prelude::*;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::config::Config;
use crate::errors::Error;

/// Converts FNIS behavior to the target format if necessary.
///
/// # Errors
/// Returns a collection of errors if any file:
/// - Cannot be read (I/O errors),
/// - Has invalid HKX magic numbers,
/// - Has a pointer size that cannot be determined.
pub(super) fn convert_behavior<'a>(
    owned_data: &'a OwnedFnisInjection,
    config: &'a Config,
) -> Result<(), Error> {
    let output_dir = config.output_dir.display().to_string();
    let output_format = config.output_target;

    let (input_path, output_inner) = owned_data
        .to_behavior_path()
        .map_err(|e| Error::FnisError { source: e })?;
    let output_path = Path::new(&output_dir).join(output_inner);

    let current_format = check_hkx_header(&input_path, output_format)?;
    if current_format == output_format {
        return Ok(());
    }

    convert_hkx(&input_path, &output_path, output_format)?;
    #[cfg(feature = "tracing")]
    tracing::info!(
        "Converted FNIS HKX file '{}' -> '{}' for target {output_format:?}",
        input_path.display(),
        output_path.display(),
    );
    Ok(())
}

/// Converts FNIS HKX animation files to the target format if necessary.
///
/// This function iterates over the provided FNIS animation file paths and checks each HKX file:
/// 1. Reads the first 16 bytes of the file to inspect the magic numbers and pointer size.
/// 2. Validates the HKX magic numbers (`0x57E0E057` and `0x10C0C010`).
/// 3. Determines the pointer size to infer the current format (`Win32` for 32-bit, `Amd64` for 64-bit).
/// 4. If the file's format matches the target format specified in `config.output_target`,
///    the file is left unchanged.
/// 5. If the file's format does not match, it is converted to the target format using the
///    FNIS conversion routine.
///
/// # Behavior
/// - Files with invalid magic numbers or headers will be reported as errors. The errors
///   include the expected values and the actual values read from the file, helping users
///   identify and fix issues (e.g., corrupted or unsupported HKX files).
/// - Files that already match the target format are skipped.
///
/// # Errors
/// Returns a collection of errors if any file:
/// - Cannot be read (I/O errors),
/// - Has invalid HKX magic numbers,
/// - Has a pointer size that cannot be determined.
#[must_use]
pub(super) fn convert_animations<'a>(
    animations: &[&'a str],
    owned_data: &'a OwnedFnisInjection,
    config: &'a Config,
) -> Vec<Error> {
    let base_dir = owned_data.behavior_entry.base_dir;
    let namespace = &owned_data.namespace;
    let output_dir = config.output_dir.display();
    let output_format = config.output_target;

    let (_, errors): ((), Vec<_>) = animations.par_iter().partition_map(|anim_file| {
        let anim_file = anim_file.replace("\\", "//");
        let input_path = owned_data.animations_mod_dir.join(&anim_file);

        let current_format = match check_hkx_header(&input_path, output_format) {
            Ok(current_format) => current_format,
            Err(err) => return Either::Right(err),
        };

        if current_format == output_format {
            return Either::Left(());
        }

        let output_path =
            format!("{output_dir}/meshes/{base_dir}/animations/{namespace}/{anim_file}");

        match convert_hkx(&input_path, Path::new(&output_path), output_format) {
            Ok(()) => {
                #[cfg(feature = "tracing")]
                tracing::info!(
                    "Converted FNIS HKX file '{}' -> '{output_path}' for target {output_format:?}",
                    input_path.display(),
                );
                Either::Left(())
            }
            Err(error) => Either::Right(error),
        }
    });

    errors
}

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
    let is_tag = {
        const TAG_MAGIC0: [u8; 4] = [0x1E, 0x0D, 0xB0, 0xCA];
        const TAG_MAGIC1: [u8; 4] = [0xCE, 0xFA, 0x11, 0xD0];
        let magic0_ok = header[0..4] == TAG_MAGIC0;
        let magic1_ok = header[4..8] == TAG_MAGIC1;
        magic0_ok && magic1_ok
    };
    if is_tag {
        #[cfg(feature = "tracing")]
        tracing::info!(
            path = %input_path.display(),
            "Tag files cannot be converted by serde_hkx, so they are skipped."
        );
        return Ok(output_format);
    }

    // check magic
    const EXPECTED_MAGIC: [u8; 8] = [
        0x57, 0xE0, 0xE0, 0x57, // magic0
        0x10, 0xC0, 0xC0, 0x10, // magic1
    ];
    if header[0..8] != EXPECTED_MAGIC {
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
) -> Result<(), Error> {
    use serde_hkx::bytes::serde::hkx_header::HkxHeader;
    use serde_hkx_features::ClassMap;

    let bytes = std::fs::read(input_path).map_err(|e| Error::FNISHkxIoError {
        path: input_path.to_path_buf(),
        target: output_format,
        source: e,
    })?;

    let class_map: ClassMap = serde_hkx::from_bytes(&bytes).map_err(|e| Error::HkxDeError {
        path: input_path.to_path_buf(),
        source: e,
    })?;

    let header = match output_format {
        crate::OutPutTarget::SkyrimLe => HkxHeader::new_skyrim_le(),
        crate::OutPutTarget::SkyrimSe => HkxHeader::new_skyrim_se(),
    };

    let bytes = serde_hkx::to_bytes(&class_map, &header).map_err(|e| Error::HkxSerError {
        path: input_path.to_path_buf(),
        source: e,
    })?;

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
