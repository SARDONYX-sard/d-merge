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
    let results: Vec<Result<(), Error>> = borrowed_patches
        .into_par_iter()
        .flat_map(|(key, patches)| apply_to_one_template(config, templates, &key, patches))
        .collect();

    filter_results(results)
}

type ChainError = rayon::iter::Chain<
    rayon::vec::IntoIter<Result<(), Error>>,
    rayon::vec::IntoIter<Result<(), Error>>,
>;

fn apply_to_one_template<'a, 'b: 'a>(
    config: &Config,
    templates: &BorrowedTemplateMap<'a>,
    key: &TemplateKey<'a>,
    patches: (OnePatchMap<'b>, SeqPatchMap<'b>),
) -> ChainError {
    if config.debug.output_patch_json {
        if let Err(err) = write_debug_json_patch(&config.output_dir, key, &patches) {
            #[cfg(feature = "tracing")]
            tracing::error!("{err}");
        }
    }

    // Single-field patches must be applied first to account for array index shifts.
    //
    // Why? Because a single-field patch can only ever be a replacement.
    // When array indices are added or removed, subsequent patches relying on specific indices
    // may break unless we apply the replacements first.
    //
    // For example:
    // - `["#0001", "hkbStringData", "eventTriggers"], value: vec![ValueWithPriority]` // add/remove index 5
    // - `["#0001", "hkbStringData", "eventTriggers", "[5]", "local_time"]`           // modify f32
    // - `["#0001", "hkbStringData", "eventTriggers", "[5]", "triggers", "[0]", "animations", "[3]", "time"]` // modify f32
    //
    // In these examples, the single-field patch (`local_time` or `time`) is always a replacement,
    // so it must be applied first. This ensures the index layout is stable
    // before applying sequence patches that manipulate array elements.
    // After that, sequence patches can be applied reliably.
    let (one_patch_map, seq_patch_map) = patches;
    let total = one_patch_map.0.len() + seq_patch_map.0.len();
    let status_reporter =
        StatusReportCounter::new(&config.status_report, ReportType::ApplyingPatches, total);

    let one_patch_results = process_one_patch(templates, key, one_patch_map, &status_reporter);
    let seq_patch_results = process_seq_patch(templates, key, seq_patch_map, &status_reporter);

    one_patch_results.into_par_iter().chain(seq_patch_results)
}

/// Processes the one_patch map.
fn process_one_patch<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    key: &TemplateKey<'a>,
    one_patch_map: OnePatchMap<'b>,
    status_reporter: &StatusReportCounter,
) -> Vec<Result<(), Error>> {
    one_patch_map
        .0
        .into_par_iter()
        .map(|(path, patch)| match templates.get_mut(key) {
            Some(mut template_pair) => {
                let template = &mut template_pair.value_mut().1;
                let result = apply_one_field(template, path, patch).with_context(|_| PatchSnafu {
                    template_name: key.to_string(),
                });
                status_reporter.increment();
                result
            }
            None => Err(Error::NotFoundTemplate {
                template_name: key.to_string(),
            }),
        })
        .collect()
}

/// Processes the seq_patch map.
fn process_seq_patch<'a, 'b: 'a>(
    templates: &BorrowedTemplateMap<'a>,
    key: &TemplateKey<'a>,
    seq_patch_map: SeqPatchMap<'b>,
    status_reporter: &StatusReportCounter,
) -> Vec<Result<(), Error>> {
    seq_patch_map
        .0
        .into_par_iter()
        .map(|(path, patches)| match templates.get_mut(key) {
            Some(mut template_pair) => {
                let template = &mut template_pair.value_mut().1;
                let template_name = key.to_string();
                let result = apply_seq_by_priority(template_name.as_str(), template, path, patches)
                    .with_context(|_| PatchSnafu { template_name });
                status_reporter.increment();
                result
            }
            None => Err(Error::NotFoundTemplate {
                template_name: key.to_string(),
            }),
        })
        .collect()
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
