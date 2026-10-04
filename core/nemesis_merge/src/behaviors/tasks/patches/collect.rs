use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
};

use json_patch::ValueWithPriority;
use nemesis_xml::patch::{PatchesMap, parse_nemesis_patch};
use rapidhash::fast::RapidHashMap;
use rayon::{iter::Either, prelude::*};
use snafu::{OptionExt as _, ResultExt as _};

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
            patches::types::{OwnedPatchMap, OwnedPatches, PatchCollection},
            templates::key::TemplateKey,
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
/// # Order
/// The paths are sorted so that the insertion order (and thus the conflict resolution of patches
/// with the same priority) is deterministic.
///
/// # Note
/// This is blocking. Many small files are read faster by `rayon` + `std::fs` than by spawning a
/// tokio task (`spawn_blocking` internally) per file.
///
/// # Errors
/// Returns an error if any of the paths cannot be read or parsed.
pub(crate) fn collect_owned_patches(
    nemesis_entries: &PriorityMap,
    config: &Config,
) -> OwnedPatches {
    fn get_priority_by_path_id(path: &Path, ids: &PriorityMap) -> Option<usize> {
        let id_str = get_nemesis_id(path.to_str()?).ok()?;
        ids.get(id_str).copied()
    }

    // NOTE: `collect_nemesis_paths` already walks in parallel(jwalk), so iterate the mods sequentially.
    let mut paths: Vec<(Category, PathBuf)> =
        nemesis_entries.keys().flat_map(collect_nemesis_paths).collect();
    paths.par_sort_unstable_by(|(_, a), (_, b)| a.cmp(b));

    let reporter =
        StatusReportCounter::new(&config.status_report, ReportType::ReadingPatches, paths.len());

    // NOTE: `collect` keeps the (sorted) order.
    let results: Vec<Result<OwnedPath>> = paths
        .into_par_iter()
        .map(|(category, path)| {
            let priority = get_priority_by_path_id(&path, nemesis_entries).unwrap_or_else(|| {
                #[cfg(feature = "tracing")]
                tracing::warn!("Not found id from path: Path({})", path.display());
                usize::MAX // todo error handling
            });

            let content = std::fs::read_to_string(&path)
                .with_context(|_| FailedIoSnafu { path: path.clone() });
            reporter.increment();

            Ok(OwnedPath { category, path, content: content?, priority })
        })
        .collect();

    let mut owned_patches = OwnedPatchMap::default();
    let mut adsf_patches = OwnedAdsfPatchMap::new();
    let mut asdsf_patches = OwnedAsdsfPatchMap::new();
    let mut errors = vec![];

    for result in results {
        match result {
            Ok(OwnedPath { category, path, content, priority }) => match category {
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

    OwnedPatches { owned_patches, adsf_patches, asdsf_patches, errors }
}

/// One parsed nemesis patch file.
struct ParsedPatchFile<'a> {
    key: TemplateKey<'static>,
    priority: usize,
    json_patches: PatchesMap<'a>,
    /// Index of `hkbBehaviorGraphData` for nemesis variable replacement.
    variable_index: Option<Cow<'static, str>>,
}

/// Parses nemesis patches and merges them into `fnis_patches`.
///
/// # Algorithm (map-reduce)
/// 1. Parse each file in parallel into a local value (no lock).
/// 2. Group the files by template in the input order (sequential, cheap).
/// 3. Insert each template group in parallel. The outer map is locked only once per template.
///
/// Because the input order is kept, the result is deterministic even for patches with the same priority.
pub(crate) fn collect_borrowed_patches<'a>(
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

    // 1/3: Parse in parallel.
    let (parsed, errors): (Vec<_>, Vec<_>) = owned_patches
        .par_iter()
        .map(|(path, (xml, priority))| -> Result<ParsedPatchFile<'a>> {
            reporter.increment();

            let (json_patches, parsed_var_index) =
                parse_nemesis_patch(xml, config.hack_options.map(Into::into))
                    .with_context(|_| NemesisXmlErrSnafu { path })?;

            let nemesis_path = parse_nemesis_path(path)?;
            let key = nemesis_path
                .to_template_key()
                .with_context(|| FailedToCastNemesisPathToTemplateKeySnafu { path })?;

            // Store variable class for nemesis variable to replace
            let variable_index =
                nemesis_path.get_variable_index().map(Cow::Borrowed).or_else(|| {
                    parsed_var_index
                        .map(|parsed_var_index| Cow::Owned(parsed_var_index.to_string()))
                });

            Ok(ParsedPatchFile { key, priority: *priority, json_patches, variable_index })
        })
        .partition_map(|result| match result {
            Ok(parsed) => Either::Left(parsed),
            Err(err) => Either::Right(err),
        });

    // 2/3: Group by template in the input order.
    let mut by_template: RapidHashMap<TemplateKey<'static>, Vec<(usize, PatchesMap<'a>)>> =
        RapidHashMap::default();
    for ParsedPatchFile { key, priority, json_patches, variable_index } in parsed {
        if let Some(variable_index) = variable_index {
            // NOTE: The first one wins (same as before, but now deterministic).
            variable_class_map.0.entry(key.clone()).or_insert(variable_index);
        }
        by_template.entry(key).or_default().push((priority, json_patches));
    }

    // 3/3: Insert per template in parallel.
    by_template.into_par_iter().for_each(|(key, files)| {
        let entry = raw_borrowed_patches.0.entry(key).or_default();

        for (priority, json_patches) in files {
            for (json_path, value) in json_patches {
                // Overwrite to match patch structure
                let is_pure = matches!(value.action, json_patch::Action::Pure { .. });
                let value = ValueWithPriority::new(value, priority);
                if is_pure {
                    entry.value().one.insert(json_path, value); // Pure: no add and remove because of single value
                } else {
                    entry.value().seq.insert(json_path, value);
                }
            }
        }
    });

    (
        PatchCollection {
            borrowed_patches: raw_borrowed_patches,
            behavior_graph_data_map: variable_class_map,
        },
        errors,
    )
}

/// Returns the template keys of the nemesis patch paths.
///
/// Only the paths are parsed(no XML), so the templates can be loaded before parsing the patches.
/// Invalid paths are skipped here. Their errors are reported by [`collect_borrowed_patches`].
pub(crate) fn template_keys_from_paths(
    owned_patches: &OwnedPatchMap,
) -> HashSet<TemplateKey<'static>, rapidhash::fast::RandomState> {
    owned_patches
        .par_iter()
        .filter_map(|(path, _)| parse_nemesis_path(path).ok()?.to_template_key())
        .collect()
}
