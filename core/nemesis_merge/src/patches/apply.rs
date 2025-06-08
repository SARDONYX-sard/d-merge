//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    errors::{Error, PatchSnafu, Result},
    results::filter_results,
    types::{BorrowedTemplateMap, Key, RawBorrowedPatches},
    Config,
};
use json_patch::apply_patch;
use rayon::prelude::*;
use snafu::ResultExt;
use std::path::Path;

/// Apply to hkx with merged json patch.
///
/// # Lifetime
/// In terms of code flow, the `patch` is longer lived than the `template`, but this inversion is achieved by
/// shrinking the lifetime of the patch by the higher-level function.
///
/// Therefore, this seemingly strange lifetime annotation is intentional.
pub fn apply_patches<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    borrowed_patches: RawBorrowedPatches<'b>,
    config: &Config,
) -> Result<(), Vec<Error>> {
    let results: Vec<Result<(), Error>> = borrowed_patches // patches
        .into_par_iter()
        .flat_map(|(key, patches)| {
            if config.debug.output_patch_json {
                if let Err(err) = write_json_patch(&config.output_dir, &key, &patches) {
                    #[cfg(feature = "tracing")]
                    tracing::error!("{err}");
                }
            }

            patches
                .0
                .into_iter()
                .map(|(path, patch)| {
                    if let Some(mut template_pair) = templates.get_mut(&key) {
                        let template = &mut template_pair.value_mut().1;
                        let template_name = key.to_string();
                        apply_patch(template_name.as_str(), template, path, patch)
                            .with_context(|_| PatchSnafu { template_name })
                    } else {
                        Err(Error::NotFoundTemplate {
                            template_name: key.to_string(),
                        })
                    }
                })
                .collect::<Vec<Result<(), Error>>>()
        })
        .collect();

    filter_results(results)
}

fn write_json_patch(
    output_dir: &Path,
    key: &Key,
    patches: &crate::types::PatchMap,
) -> Result<(), Error> {
    use crate::errors::FailedIoSnafu;
    use snafu::ResultExt as _;

    let output_dir = if key.is_1st_person {
        let output_dir_1st_person = output_dir.join(".debug").join("patches").join("_1stperson");
        std::fs::create_dir_all(&output_dir_1st_person).context(FailedIoSnafu {
            path: output_dir_1st_person.clone(),
        })?;
        output_dir_1st_person
    } else {
        output_dir.join(".debug").join("patches")
    };

    std::fs::create_dir_all(&output_dir).context(FailedIoSnafu {
        path: output_dir.clone(),
    })?;

    let output_path = output_dir.join(format!("{}.patch.json", key.template_name));
    let json = simd_json::to_string_pretty(patches).with_context(|_| crate::errors::JsonSnafu {
        path: output_path.clone(),
    })?;
    std::fs::write(&output_path, &json).context(FailedIoSnafu { path: output_path })?;

    Ok(())
}
