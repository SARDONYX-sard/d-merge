//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    behaviors::tasks::{
        patches::types::{OnePatchMap, RawBorrowedPatches, SeqPatchMap},
        templates::types::{BorrowedTemplateMap, TemplateKey},
    },
    config::{ReportType, StatusReportCounter},
    errors::{Error, PatchSnafu, Result},
    results::filter_results,
    Config,
};
use json_patch::{apply_one_field, apply_seq_by_priority};
use rayon::prelude::*;
use simd_json::borrowed::Value;
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
    templates: &mut BorrowedTemplateMap<'a>,
    borrowed_patches: RawBorrowedPatches<'b>,
    config: &Config,
) -> Result<(), Vec<Error>> {
    let status_report = &config.status_report;
    // Optimization: If we don't use the progress bar, there is no need to calculate.
    let total = match status_report {
        Some(_) => borrowed_patches.len(),
        None => 0,
    };

    let status_reporter =
        StatusReportCounter::new(status_report, ReportType::ApplyingPatches, total);

    // Step 1: Remove templates and build working set
    let working_set: Vec<_> = borrowed_patches
        .0
        .into_iter()
        .filter_map(|(key, patches)| {
            templates
                .remove(&key)
                .map(|(_, template)| (key, patches, template))
        })
        .collect();

    // Step 2: Apply patches in parallel
    let (results, updated_templates): (Vec<_>, Vec<_>) = working_set
        .into_par_iter()
        .map(|(key, patches, (template_name, mut template_value))| {
            let patch_results = apply_to_one_template(
                config,
                &key,
                template_name,
                &mut template_value,
                patches,
                &status_reporter,
            );
            (patch_results, (key, (template_name, template_value)))
        })
        .unzip();

    // Step 3: Put patched templates back
    templates.par_extend(updated_templates);

    // Step 4: Return aggregated results
    let flat: Vec<_> = results.into_par_iter().flatten().collect();
    filter_results(flat)
}

/// Applies one-field and sequence patches to a single template.
///
/// # Returns
/// Parallel iterator of patch results (success or error).
fn apply_to_one_template<'a, 'b: 'a>(
    config: &Config,
    key: &TemplateKey<'a>,
    template_name: &'a str,
    template_value: &mut Value<'a>,
    patches: (OnePatchMap<'b>, SeqPatchMap<'b>),
    status_reporter: &StatusReportCounter,
) -> Vec<Result<(), Error>> {
    if config.debug.output_patch_json {
        if let Err(err) = write_debug_json_patch(&config.output_dir, key, &patches) {
            #[cfg(feature = "tracing")]
            tracing::error!("{err}");
        }
    }

    let (one_patch_map, seq_patch_map) = patches;

    let mut results = Vec::with_capacity(one_patch_map.0.len() + seq_patch_map.0.len());

    // NOTE: Why not use par_iter here?
    // Since the template change targets overlap, locking with Arc<Mutex<T>> will likely slow things down.
    for (path, patch) in one_patch_map.0 {
        let result = apply_one_field(template_value, path, patch).with_context(|_| PatchSnafu {
            template_name: key.to_string(),
        });
        status_reporter.increment();
        results.push(result);
    }

    for (path, patches) in seq_patch_map.0 {
        let result = apply_seq_by_priority(template_name, template_value, path, patches)
            .with_context(|_| PatchSnafu {
                template_name: template_name.to_string(),
            });
        status_reporter.increment();
        results.push(result);
    }

    results
}

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn write_debug_json_patch(
    output_dir: &Path,
    key: &TemplateKey,
    patches: &(OnePatchMap, SeqPatchMap),
) -> Result<(), Error> {
    use crate::errors::FailedIoSnafu;
    use snafu::ResultExt as _;

    let output_dir = if key.is_1st_person {
        let output_dir_1st_person = output_dir
            .join(".d_merge")
            .join(".debug")
            .join("patches")
            .join("_1stperson");
        std::fs::create_dir_all(&output_dir_1st_person).context(FailedIoSnafu {
            path: output_dir_1st_person.clone(),
        })?;
        output_dir_1st_person
    } else {
        output_dir.join(".d_merge").join(".debug").join("patches")
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
