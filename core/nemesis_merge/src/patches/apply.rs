//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    errors::{Error, PatchSnafu, Result},
    results::filter_results,
    types::{BorrowedTemplateMap, Key, RawBorrowedPatches},
    Config,
};
use json_patch::{
    apply_one_field, apply_seq_by_priority,
    json_path::nested_path::{sort_nested_json_path, PathType},
    Patch,
};
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
pub fn apply_patches<'a>(
    templates: &BorrowedTemplateMap<'a>,
    borrowed_patches: RawBorrowedPatches<'a>,
    config: &Config,
) -> Result<(), Vec<Error>> {
    let results: Vec<Result<(), Error>> = borrowed_patches
        .into_par_iter()
        .flat_map(|(key, patches)| {
            if config.debug.output_patch_json {
                if let Err(err) = write_json_patch(&config.output_dir, &key, &patches) {
                    #[cfg(feature = "tracing")]
                    tracing::error!("{err}");
                }
            }

            // 1/4: grouping_nested_patch
            let path_types = {
                let entries = patches.0.iter().map(|r| r.key().clone()).collect();
                sort_nested_json_path(entries)
            };

            let mut results = vec![];
            for path_type in path_types {
                match path_type {
                    PathType::Simple(path) => {
                        let Some((path, patch)) = patches.0.remove(&path) else {
                            results.push(Err(Error::Custom {
                                msg: format!("not found this path patch: {path:?}"),
                            }));
                            continue;
                        };
                        let patch = match patch {
                            json_patch::Patch::One(patch) => patch,
                            json_patch::Patch::Seq(_) => {
                                results.push(Err(Error::Custom {
                                    msg: "Expected One patch but got seq".to_string(),
                                }));
                                continue;
                            }
                        };

                        match templates.get_mut(&key) {
                            Some(mut template_pair) => {
                                let template = &mut template_pair.value_mut().1;
                                let template_name = key.to_string();
                                results.push(
                                    apply_one_field(template, path, patch)
                                        .with_context(|_| PatchSnafu { template_name }),
                                );
                            }
                            None => results.push(Err(Error::NotFoundTemplate {
                                template_name: key.to_string(),
                            })),
                        }
                    }
                    PathType::Nested((base, children)) => {
                        // paths = vec![
                        //    ["#0001", "hkbStringData", "eventTriggers"], value: vec![ValueWithPriority] -> replace-> one patch -> add&remove
                        //    ["#0001", "hkbStringData", "eventTriggers", "[5]", "local_time"] // modify f32
                        //    ["#0001", "hkbStringData", "eventTriggers", "[5]", "triggers", [0], "animations", [3], "time"] // modify f32
                        // ]

                        // ["#0001", "hkbStringData", "eventTriggers"], value: vec![ValueWithPriority] -> replace-> one patch -> add&remove
                        let Some((path, patch)) = patches.0.remove(&base) else {
                            results.push(Err(Error::Custom {
                                msg: format!("not found this path patch base: {base:?}"),
                            }));
                            continue;
                        };
                        let Some(mut template_pair) = templates.get_mut(&key) else {
                            results.push(Err(Error::NotFoundTemplate {
                                template_name: key.to_string(),
                            }));
                            continue;
                        };
                        let mut child_patches = vec![];
                        for (full_child_path, child) in &children {
                            let Some((path, Patch::One(patch))) = patches.0.remove(full_child_path)
                            else {
                                results.push(Err(Error::Custom {
                                    msg: format!("not found this path patch child: {child:?}"),
                                }));
                                continue;
                            };

                            child_patches.push((path, patch));
                        }

                        let patches = match patch {
                            Patch::One(patch) => {
                                results.push(Err(Error::Custom {
                                    msg: format!("Expected One patch but got Seq:\n{patch:#?}"),
                                }));
                                continue;
                            }
                            Patch::Seq(patches) => patches,
                        };

                        let template = &mut template_pair.value_mut().1;
                        let template_name = key.to_string();

                        if let Err(err) = apply_seq_by_priority(
                            &template_name,
                            template,
                            &path,
                            patches,
                            child_patches,
                        )
                        .with_context(|_| PatchSnafu {
                            template_name: template_name.clone(),
                        }) {
                            results.push(Err(err));
                        };
                    }
                }
            }

            results
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
