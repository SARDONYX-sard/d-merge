#![allow(clippy::significant_drop_tightening)]
use super::paths::{
    collect::{collect_nemesis_paths, Category},
    parse::parse_nemesis_path,
};
use crate::{
    errors::{Error, FailedIoSnafu, NemesisXmlErrSnafu, Result},
    path_id::get_nemesis_id,
    results::filter_results,
    types::{
        OwnedAdsfPatchMap, OwnedPatchMap, PatchKind, PatchMap, PatchMapForEachTemplate,
        PriorityMap, VariableClassMap,
    },
};
use dashmap::DashSet;
use json_patch::ValueWithPriority;
use nemesis_xml::{hack::HackOptions, patch::parse_nemesis_patch};
use rayon::prelude::*;
use snafu::ResultExt as _;
use std::path::{Path, PathBuf};
use tokio::fs;

struct OwnedPath {
    category: Category,
    path: PathBuf,
    content: String,
    priority: usize,
}

/// Collects all patches from the given nemesis paths and returns a map of owned patches.
///
///
/// - e.g. path: `/some/path/to/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt`
///
/// # Errors
/// Returns an error if any of the paths cannot be read or parsed.
pub async fn collect_owned_patches(
    nemesis_paths: &[PathBuf],
    id_order: &PriorityMap<'_>,
) -> (OwnedAdsfPatchMap, OwnedPatchMap, Vec<Error>) {
    let mut handles = vec![];
    let paths = nemesis_paths.iter().flat_map(collect_nemesis_paths);

    fn get_priority(path: &Path, ids: &PriorityMap<'_>) -> Option<usize> {
        let id_str = get_nemesis_id(path.to_str()?).ok()?;
        ids.get(id_str).copied()
    }

    for (category, path) in paths {
        let priority = get_priority(&path, id_order).unwrap_or(usize::MAX); // todo error handling

        handles.push(tokio::spawn(async move {
            let content = fs::read_to_string(&path)
                .await
                .with_context(|_| FailedIoSnafu { path: path.clone() })?;

            Ok(OwnedPath {
                category,
                path,
                content,
                priority,
            })
        }));
    }

    let mut owned_patches = OwnedPatchMap::new();
    let mut adsf_patches = OwnedAdsfPatchMap::new();
    let mut errors = vec![];

    for handle in handles {
        let result = match handle.await {
            Ok(result) => result,
            Err(err) => {
                errors.push(Error::JoinError { source: err });
                continue;
            }
        };

        match result {
            Ok(OwnedPath {
                category,
                path,
                content,
                priority,
            }) => match category {
                Category::Nemesis => {
                    owned_patches.insert(path, (content, priority));
                }
                Category::Adsf => {
                    adsf_patches.insert(path, (content, priority));
                }
            },
            Err(err) => {
                errors.push(err);
            }
        }
    }

    (adsf_patches, owned_patches, errors)
}

pub fn collect_borrowed_patches<'a>(
    owned_patches: &'a OwnedPatchMap,
    hack_options: Option<HackOptions>,
) -> (BorrowedPatches<'a>, Vec<Error>) {
    let patch_map_foreach_template = PatchMapForEachTemplate::new();
    let template_names = DashSet::new();
    let variable_class_map = VariableClassMap::new();

    let results: Vec<Result<()>> = owned_patches
        .par_iter()
        .map(|(path, (xml, priority))| {
            // Since we could not make a destructing assignment, we have to write it this way.
            let priority = *priority;

            let (json_patches, variable_class_index) = parse_nemesis_patch(xml, hack_options)
                .with_context(|_| NemesisXmlErrSnafu { path })?;

            let template_name = parse_nemesis_path(path)?; // Store variable class for nemesis variable to replace

            template_names.insert(template_name);
            if let Some(class_index) = variable_class_index {
                variable_class_map
                    .0
                    .entry(template_name)
                    .or_insert(class_index);
            }

            json_patches.into_par_iter().for_each(|(key, value)| {
                // FIXME: I think that if we lengthen the lock period, we can suppress the race condition, but that will slow down the process.
                let entry = patch_map_foreach_template
                    .entry(template_name)
                    .or_insert_with(PatchMap::new);

                match &value.op {
                    // Overwrite to match patch structure
                    // Pure: no add and remove because of single value
                    json_patch::OpRangeKind::Pure(_) => {
                        let value = ValueWithPriority::new(value, priority);
                        let _ = entry.value().insert(key, value, PatchKind::OneField);
                    }
                    json_patch::OpRangeKind::Seq(_) => {
                        let value = ValueWithPriority::new(value, priority);
                        let _ = entry.value().insert(key, value, PatchKind::Seq);
                    }
                    json_patch::OpRangeKind::Discrete(range_vec) => {
                        let json_patch::JsonPatch { value, .. } = value;

                        let array =
                            match simd_json::derived::ValueTryIntoArray::try_into_array(value) {
                                Ok(array) => array,
                                Err(_err) => {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("{_err}",);
                                    return;
                                }
                            };

                        let iter =
                            range_vec
                                .clone()
                                .into_par_iter()
                                .zip(array)
                                .map(|(range, value)| {
                                    let value = json_patch::JsonPatch {
                                        op: json_patch::OpRangeKind::Seq(range),
                                        value,
                                    };
                                    let value = ValueWithPriority::new(value, priority);
                                    value
                                });
                        let _ = entry.value().extend(key, iter);
                    }
                }
            });

            Ok(())
        })
        .collect();

    let errors = match filter_results(results) {
        Ok(()) => vec![],
        Err(errors) => errors,
    };

    (
        BorrowedPatches {
            template_names,
            patch_map_foreach_template,
            variable_class_map,
        },
        errors,
    )
}

pub struct BorrowedPatches<'a> {
    /// Name of the template that needs to be read.
    pub template_names: DashSet<&'a str>,
    /// - key: template name (e.g., `"0_master"`, `"defaultmale"`)
    /// - value: `Map<jsonPath, { patch, priority }>`
    pub patch_map_foreach_template: PatchMapForEachTemplate<'a>,
    /// HashMap showing which index (e.g. `#0000`) of each template (e.g. `0_master.xml`)
    /// contains `hkbBehaviorGraphStringData
    ///
    /// This information exists because it is needed to replace variables such as the Nemesis variable `$variableID[]$`.
    pub variable_class_map: VariableClassMap<'a>,
}
