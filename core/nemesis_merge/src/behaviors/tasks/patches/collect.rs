use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use json_patch::ValueWithPriority;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::{iter::Either, prelude::*};
use snafu::{OptionExt as _, ResultExt as _};
use tokio::fs;

use super::paths::{
    collect::{Category, collect_nemesis_paths},
    parse::parse_nemesis_path,
};
use crate::{
    Config,
    behaviors::{
        priority_ids::{get_nemesis_id, types::PriorityMap},
        tasks::{
            adsf::types::OwnedAdsfPatchMap,
            asdsf::types::OwnedAsdsfPatchMap,
            patches::types::{
                BehaviorGraphDataMap, BehaviorPatchesMap, OwnedPatchMap, OwnedPatches,
                PatchCollection,
            },
        },
    },
    config::{ReportType, StatusReportCounter},
    errors::{
        Error, FailedIoSnafu, FailedToCastNemesisPathToTemplateKeySnafu, NemesisXmlErrSnafu, Result,
    },
};

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

struct LocalAgg<'a> {
    nemesis_borrowed_patches: BehaviorPatchesMap<'a>,
    nemesis_variable_class_map: BehaviorGraphDataMap<'a>,
}

impl<'a> LocalAgg<'a> {
    fn new() -> Self {
        Self {
            nemesis_borrowed_patches: BehaviorPatchesMap::default(),
            nemesis_variable_class_map: BehaviorGraphDataMap::default(),
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.nemesis_borrowed_patches
            .merge(other.nemesis_borrowed_patches);
        self.nemesis_variable_class_map
            .0
            .par_extend(other.nemesis_variable_class_map.0);
        self
    }
}

pub fn collect_borrowed_patches<'a>(
    owned_patches: &'a OwnedPatchMap,
    config: &Config,
    mut fnis_patches: PatchCollection<'a>,
) -> (PatchCollection<'a>, Vec<Error>) {
    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::ParsingPatches,
        owned_patches.len(),
    );

    let (locals, errors): (Vec<LocalAgg<'a>>, Vec<Error>) = owned_patches
        .par_iter()
        .map(|(path, (xml, priority))| -> Result<LocalAgg<'a>> {
            reporter.increment();
            let priority = *priority;

            let (json_patches, parsed_var_index) =
                parse_nemesis_patch(xml, config.hack_options.map(Into::into))
                    .with_context(|_| NemesisXmlErrSnafu { path })?;

            let nemesis_path = parse_nemesis_path(path)?;
            let key = nemesis_path
                .to_template_key()
                .with_context(|| FailedToCastNemesisPathToTemplateKeySnafu { path })?;

            let mut agg = LocalAgg::new();

            if let Some(master_behavior_graph_index) = nemesis_path.get_variable_index() {
                agg.nemesis_variable_class_map
                    .0
                    .entry(key.clone())
                    .or_insert(Cow::Borrowed(master_behavior_graph_index));
            } else if let Some(parsed_var_index) = parsed_var_index {
                agg.nemesis_variable_class_map
                    .0
                    .entry(key.clone())
                    .or_insert_with(|| Cow::Owned(parsed_var_index.to_string()));
            }

            for (json_path, value) in json_patches {
                let entry = agg
                    .nemesis_borrowed_patches
                    .0
                    .entry(key.clone())
                    .or_default();
                match &value.action {
                    json_patch::Action::Pure { .. } => {
                        entry
                            .one
                            .insert(json_path, ValueWithPriority::new(value, priority));
                    }
                    json_patch::Action::Seq { .. } | json_patch::Action::SeqPush => {
                        entry
                            .seq
                            .insert(json_path, ValueWithPriority::new(value, priority));
                    }
                }
            }

            Ok(agg)
        })
        .partition_map(|r| match r {
            Ok(agg) => Either::Left(agg),
            Err(e) => Either::Right(e),
        });

    let LocalAgg {
        nemesis_borrowed_patches,
        nemesis_variable_class_map,
    } = locals
        .into_par_iter()
        .reduce(LocalAgg::new, |a, b| a.merge(b));

    // Merge nemesis results on top of the fnis base
    fnis_patches
        .borrowed_patches
        .merge(nemesis_borrowed_patches);
    fnis_patches
        .behavior_graph_data_map
        .0
        .par_extend(nemesis_variable_class_map.0);

    (fnis_patches, errors)
}
