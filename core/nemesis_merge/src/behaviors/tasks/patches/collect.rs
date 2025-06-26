use super::paths::{
    collect::{collect_nemesis_paths, Category},
    parse::parse_nemesis_path,
};
use crate::{
    behaviors::{
        priority_ids::{get_nemesis_id, types::PriorityMap},
        tasks::{
            adsf::types::OwnedAdsfPatchMap,
            asdsf::types::OwnedAsdsfPatchMap,
            patches::types::{
                BehaviorStringDataMap, BorrowedPatches, OwnedPatchMap, OwnedPatches,
                RawBorrowedPatches,
            },
            templates::types::TemplateKey,
        },
    },
    config::{ReportType, StatusReportCounter},
    errors::{Error, FailedIoSnafu, NemesisXmlErrSnafu, Result},
    results::filter_results,
    Config,
};
use dashmap::DashSet;
use json_patch::ValueWithPriority;
use nemesis_xml::patch::parse_nemesis_patch;
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
    config: &Config,
) -> OwnedPatches {
    let mut handles = vec![];
    let paths = nemesis_paths.iter().flat_map(collect_nemesis_paths);

    fn get_priority_by_path_id(path: &Path, ids: &PriorityMap<'_>) -> Option<usize> {
        let id_str = get_nemesis_id(path.to_str()?).ok()?;
        ids.get(id_str).copied()
    }

    for (category, path) in paths {
        let priority = get_priority_by_path_id(&path, id_order).unwrap_or(usize::MAX); // todo error handling

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
    let mut asdsf_patches = OwnedAsdsfPatchMap::new();
    let mut errors = vec![];

    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::ReadingPatches,
        handles.len(),
    );

    for handle in handles {
        reporter.increment();

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
                Category::Asdsf => {
                    asdsf_patches.insert(path, (content, priority));
                }
            },
            Err(err) => {
                errors.push(err);
            }
        }
    }

    OwnedPatches {
        owned_patches,
        adsf_patches,
        asdsf_patches,
        errors,
    }
}

pub fn collect_borrowed_patches<'a>(
    owned_patches: &'a OwnedPatchMap,
    config: &Config,
) -> (BorrowedPatches<'a>, Vec<Error>) {
    let raw_borrowed_patches = RawBorrowedPatches::default();
    let template_names = DashSet::new();
    let variable_class_map = BehaviorStringDataMap::new();

    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::ParsingPatches,
        owned_patches.len(),
    );

    let results: Vec<Result<()>> =
        owned_patches
            .par_iter()
            .map(|(path, (xml, priority))| {
                reporter.increment();
                // Since we could not make a destructing assignment, we have to write it this way.
                let priority = *priority;

                let (json_patches, variable_class_index) =
                    parse_nemesis_patch(xml, config.hack_options.map(Into::into))
                        .with_context(|_| NemesisXmlErrSnafu { path })?;

                let (template_name, is_1st_person) = parse_nemesis_path(path)?; // Store variable class for nemesis variable to replace
                let key = TemplateKey::new(template_name, is_1st_person);

                template_names.insert(key.clone());
                if let Some(class_index) = variable_class_index {
                    variable_class_map
                        .0
                        .entry(key.clone())
                        .or_insert(class_index);
                }

                json_patches.into_par_iter().for_each(|(json_path, value)| {
                    // FIXME: I think that if we lengthen the lock period, we can suppress the race condition, but that will slow down the process.
                    let entry = raw_borrowed_patches.0.entry(key.clone()).or_default();

                    match &value.op {
                        // Overwrite to match patch structure
                        // Pure: no add and remove because of single value
                        json_patch::OpRangeKind::Pure(_) => {
                            let value = ValueWithPriority::new(value, priority);
                            entry.value().0.insert(json_path, value);
                        }
                        json_patch::OpRangeKind::Seq(_) => {
                            let value = ValueWithPriority::new(value, priority);
                            entry.value().1.insert(json_path, value);
                        }
                        json_patch::OpRangeKind::Discrete(range_vec) => {
                            let json_patch::JsonPatch { value, .. } = value;

                            let array = match simd_json::derived::ValueTryIntoArray::try_into_array(
                                value,
                            ) {
                                Ok(array) => array,
                                Err(_err) => {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("{_err}",);
                                    return;
                                }
                            };

                            let iter = range_vec.clone().into_par_iter().zip(array).map(
                                |(range, value)| {
                                    let value = json_patch::JsonPatch {
                                        op: json_patch::OpRangeKind::Seq(range),
                                        value,
                                    };
                                    let value = ValueWithPriority::new(value, priority);
                                    value
                                },
                            );
                            entry.value().1.extend(json_path, iter);
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
            borrowed_patches: raw_borrowed_patches,
            behavior_string_data_map: variable_class_map,
        },
        errors,
    )
}
