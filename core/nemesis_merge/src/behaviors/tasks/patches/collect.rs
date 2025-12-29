use json_patch::ValueWithPriority;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::{OptionExt as _, ResultExt as _};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
use tokio::fs;

use super::paths::{
    collect::{collect_nemesis_paths, Category},
    parse::parse_nemesis_path,
};
use crate::behaviors::priority_ids::{get_nemesis_id, types::PriorityMap};
use crate::behaviors::tasks::{
    adsf::types::OwnedAdsfPatchMap,
    asdsf::types::OwnedAsdsfPatchMap,
    patches::types::{OwnedPatchMap, OwnedPatches, PatchCollection},
};
use crate::config::{ReportType, StatusReportCounter};
use crate::errors::{
    Error, FailedIoSnafu, FailedToCastNemesisPathToTemplateKeySnafu, NemesisXmlErrSnafu, Result,
};
use crate::results::filter_results;
use crate::Config;

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
pub async fn collect_owned_patches(nemesis_entries: &PriorityMap, config: &Config) -> OwnedPatches {
    fn get_priority_by_path_id(path: &Path, ids: &PriorityMap) -> Option<usize> {
        let id_str = get_nemesis_id(path.to_str()?).ok()?;
        ids.get(id_str).copied()
    }

    let paths = nemesis_entries
        .iter()
        .flat_map(|(path, _)| collect_nemesis_paths(path));

    let mut handles = tokio::task::JoinSet::new();
    for (category, path) in paths {
        let priority = get_priority_by_path_id(&path, nemesis_entries).unwrap_or(usize::MAX); // todo error handling

        handles.spawn(async move {
            let content = fs::read_to_string(&path)
                .await
                .with_context(|_| FailedIoSnafu { path: path.clone() })?;

            Ok(OwnedPath {
                category,
                path,
                content,
                priority,
            })
        });
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

    while let Some(result) = handles.join_next().await {
        reporter.increment();

        let result = match result {
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
    fnis_patches: PatchCollection<'a>,
) -> (PatchCollection<'a>, Vec<Error>) {
    let PatchCollection {
        borrowed_patches: raw_borrowed_patches,
        behavior_graph_data_map: variable_class_map,
    } = fnis_patches;

    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::ParsingPatches,
        owned_patches.len(),
    );

    let results: Vec<Result<()>> = owned_patches
        .par_iter()
        .map(|(path, (xml, priority))| {
            reporter.increment();
            // Since we could not make a destructing assignment, we have to write it this way.
            let priority = *priority;

            let (json_patches, parsed_var_index) =
                parse_nemesis_patch(xml, config.hack_options.map(Into::into))
                    .with_context(|_| NemesisXmlErrSnafu { path })?;

            let key = {
                let nemesis_path = parse_nemesis_path(path)?;

                let key = nemesis_path
                    .to_template_key()
                    .with_context(|| FailedToCastNemesisPathToTemplateKeySnafu { path })?;

                // Store variable class for nemesis variable to replace
                if let Some(master_behavior_graph_index) = nemesis_path.get_variable_index() {
                    variable_class_map
                        .0
                        .entry(key.clone())
                        .or_insert(Cow::Borrowed(master_behavior_graph_index));
                } else if let Some(parsed_var_index) = parsed_var_index {
                    variable_class_map
                        .0
                        .entry(key.clone())
                        .or_insert(Cow::Owned(parsed_var_index.to_string()));
                }

                key
            };

            json_patches.into_par_iter().for_each(|(json_path, value)| {
                // FIXME: I think that if we lengthen the lock period, we can suppress the race condition, but that will slow down the process.
                let entry = raw_borrowed_patches.0.entry(key.clone()).or_default();

                // Overwrite to match patch structure
                match &value.action {
                    json_patch::Action::Pure { .. } => {
                        let value = ValueWithPriority::new(value, priority);
                        entry.value().one.insert(json_path, value); // Pure: no add and remove because of single value
                    }
                    json_patch::Action::Seq { .. } | json_patch::Action::SeqPush => {
                        let value = ValueWithPriority::new(value, priority);
                        entry.value().seq.insert(json_path, value);
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
        PatchCollection {
            borrowed_patches: raw_borrowed_patches,
            behavior_graph_data_map: variable_class_map,
        },
        errors,
    )
}
