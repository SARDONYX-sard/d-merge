//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use std::{borrow::Cow, path::Path};

use json_patch::{JsonPath, ValueWithPriority, apply_one_field, apply_seq_by_priority};
use rapidhash::fast::RapidHashMap;
use rayon::prelude::*;
use simd_json::borrowed::{Object, Value};
use snafu::ResultExt;

use crate::{
    Config,
    behaviors::tasks::{
        patches::types::{BehaviorPatchesMap, HkxPatchMaps},
        templates::{key::TemplateKey, types::BorrowedTemplateMap},
    },
    config::{ReportType, StatusReportCounter},
    errors::{Error, PatchSnafu, Result},
};

/// Apply to hkx with merged json patch.
///
/// # Lifetime
/// In terms of code flow, the `patch` is longer lived than the `template`, but this inversion is achieved by
/// shrinking the lifetime of the patch by the higher-level function.
///
/// Therefore, this seemingly strange lifetime annotation is intentional.
pub(crate) fn apply_patches<'t, 'p: 't>(
    templates: &mut BorrowedTemplateMap<'t>,
    borrowed_patches: BehaviorPatchesMap<'p>,
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
            templates.remove(&key).map(|(_, template)| (key, patches, template))
        })
        .collect();

    // Step 2: Apply patches in parallel
    let (results, updated_templates): (Vec<_>, Vec<_>) = working_set
        .into_par_iter()
        .map(|(key, patches, mut template_value)| {
            let patch_results =
                apply_to_one_template(config, &key, &mut template_value, patches, &status_reporter);
            (patch_results, (key, template_value))
        })
        .unzip();

    // Step 3: Put patched templates back
    templates.par_extend(updated_templates);

    // Step 4: Return aggregated errors
    let errors: Vec<Error> = results.into_iter().flatten().collect();
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Patches targeting one class(`path[0]`, e.g. `#0001`).
#[derive(Default)]
struct ClassPatches<'a> {
    one: Vec<(JsonPath<'a>, ValueWithPriority<'a>)>,
    seq: Vec<(JsonPath<'a>, Vec<ValueWithPriority<'a>>)>,
}

/// Applies one-field and sequence patches to a single template.
///
/// # Parallelism
/// The top level of a template is `{ "#0001": class, ... }`, and every patch path starts with the class id.
/// Patches for different classes are independent, so they are applied per class in parallel.
/// (Large templates such as `0_master` used to be patched by a single thread.)
///
/// 1. Class-level one-field patches(`path.len() <= 2`, e.g. adding a class) change the top level keys,
///    so they are applied first, sequentially.
/// 2. The other patches are grouped by class id and applied in parallel. Per class: one-field -> sequence(same as before).
/// 3. Patches for a class not found in the template are applied to the whole template as before,
///    so they report the same errors.
///
/// # Returns
/// Errors of patches.
fn apply_to_one_template<'a, 'b: 'a>(
    config: &Config,
    key: &TemplateKey<'a>,
    template_value: &mut Value<'a>,
    patches: HkxPatchMaps<'b>,
    status_reporter: &StatusReportCounter,
) -> Vec<Error> {
    if config.debug.output_patch_json
        && let Err(err) = write_debug_json_patch(&config.output_dir, key, &patches)
    {
        #[cfg(feature = "tracing")]
        tracing::error!("{err}");
    }

    let HkxPatchMaps { one: one_patch_map, seq: seq_patch_map } = patches;

    let mut class_level = ClassPatches::default();
    let mut top_level_seq = ClassPatches::default();
    let mut by_class: RapidHashMap<Cow<'b, str>, ClassPatches<'b>> = RapidHashMap::default();
    for (path, patch) in one_patch_map.into_inner() {
        match path.first() {
            Some(class_id) if path.len() > 2 => {
                by_class.entry(class_id.clone()).or_default().one.push((path, patch));
            }
            _ => class_level.one.push((path, patch)),
        }
    }
    for (path, patches) in seq_patch_map.0 {
        match path.first() {
            Some(class_id) if path.len() > 2 => {
                by_class.entry(class_id.clone()).or_default().seq.push((path, patches));
            }
            _ => top_level_seq.seq.push((path, patches)),
        }
    }

    // 1/3: Class-level patches.
    let mut errors = apply_class_patches(key, template_value, class_level, status_reporter);

    // 2/3: Per class in parallel.
    if let Value::Object(classes) = template_value {
        let targets: Vec<_> = classes
            .iter_mut()
            .filter_map(|(class_id, class)| {
                let patches = by_class.remove(class_id.as_ref())?;
                Some((class_id.clone(), class, patches))
            })
            .collect();

        errors.par_extend(targets.into_par_iter().flat_map_iter(|(class_id, class, patches)| {
            apply_to_one_class(key, class_id, class, patches, status_reporter)
        }));
    }

    // 3/3: The rest(Missing classes, or the template is not an object).
    for (_, patches) in by_class {
        errors.extend(apply_class_patches(key, template_value, patches, status_reporter));
    }
    errors.extend(apply_class_patches(key, template_value, top_level_seq, status_reporter));

    errors
}

/// Applies `patches` to one class.
///
/// The class is temporarily wrapped as `{ class_id: class }`, so that the patches can be applied
/// with their full json path(the error messages stay the same as patching the whole template).
fn apply_to_one_class<'a, 'b: 'a>(
    key: &TemplateKey<'_>,
    class_id: Cow<'a, str>,
    class: &mut Value<'a>,
    patches: ClassPatches<'b>,
    status_reporter: &StatusReportCounter,
) -> Vec<Error> {
    let mut wrapper = Object::default();
    wrapper.insert(class_id.clone(), core::mem::take(class));
    let mut wrapper = Value::Object(Box::new(wrapper));

    let errors = apply_class_patches(key, &mut wrapper, patches, status_reporter);

    // NOTE: Patches for a class have `path.len() > 2`, so the class itself is never removed.
    if let Value::Object(mut wrapper) = wrapper
        && let Some(patched) = wrapper.remove(class_id.as_ref())
    {
        *class = patched;
    }
    errors
}

/// Applies one-field patches, then sequence patches to `json`.
fn apply_class_patches<'a, 'b: 'a>(
    key: &TemplateKey<'_>,
    json: &mut Value<'a>,
    patches: ClassPatches<'b>,
    status_reporter: &StatusReportCounter,
) -> Vec<Error> {
    let ClassPatches { one, seq } = patches;
    let mut errors = vec![];

    for (path, patch) in one {
        if let Err(err) = apply_one_field(json, path, patch)
            .with_context(|_| PatchSnafu { template_name: key.to_string() })
        {
            errors.push(err);
        }
        status_reporter.increment();
    }

    for (path, patches) in seq {
        if let Err(err) = apply_seq_by_priority(key.as_str(), json, path, patches)
            .with_context(|_| PatchSnafu { template_name: key.to_string() })
        {
            errors.push(err);
        }
        status_reporter.increment();
    }

    errors
}

/////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn write_debug_json_patch(
    output_dir: &Path,
    key: &TemplateKey,
    patches: &HkxPatchMaps,
) -> Result<(), Error> {
    use snafu::ResultExt as _;

    use crate::errors::FailedIoSnafu;

    let mut output_path =
        output_dir.join(".d_merge").join(".debug").join("patches").join(key.as_meshes_inner_path());
    output_path.set_extension("patch.json");

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).context(FailedIoSnafu { path: parent })?;
    }

    let json = sonic_rs::to_string_pretty(patches)
        .with_context(|_| crate::errors::JsonSnafu { path: output_path.clone() })?;
    std::fs::write(&output_path, &json).context(FailedIoSnafu { path: output_path })?;

    Ok(())
}
