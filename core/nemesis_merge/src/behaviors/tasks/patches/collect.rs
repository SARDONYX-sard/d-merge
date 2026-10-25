use std::{
    borrow::Cow,
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
            patches::types::{
                BehaviorPatchesMap, HkxPatchMaps, OwnedPatchMap, OwnedPatches, PatchCollection,
            },
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

    let mut paths: Vec<_> = nemesis_entries
        .par_iter()
        .flat_map_iter(|(mod_dir, _)| collect_nemesis_paths(mod_dir))
        .collect();
    // Fix the order so that the output does not depend on the walk/hash order.
    paths.par_sort_unstable_by(|(_, a), (_, b)| a.cmp(b));

    let reporter =
        StatusReportCounter::new(&config.status_report, ReportType::ReadingPatches, paths.len());

    // NOTE: For thousands of small files, blocking reads on rayon are much cheaper than
    // spawning a tokio task (+ `spawn_blocking`) per file.
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

    // 1/3: Parse each file in parallel into thread-local data (no shared map, no lock).
    //
    // NOTE: `partition_map` into `Vec` keeps the input order, so the later steps are deterministic.
    let (parsed, errors): (Vec<ParsedPatch<'a>>, Vec<Error>) = owned_patches
        .par_iter()
        .map(|(path, (xml, priority))| {
            reporter.increment();
            parse_one_patch(path, xml, *priority, config)
        })
        .partition_map(|result| match result {
            Ok(parsed) => Either::Left(parsed),
            Err(err) => Either::Right(err),
        });

    // 2/3: Group by template (sequential: only moves `Vec`s).
    let mut by_template: RapidHashMap<TemplateKey<'static>, TemplatePatches<'a>> =
        RapidHashMap::default();
    for (key, maps) in raw_borrowed_patches.0 {
        by_template.entry(key).or_default().base = maps;
    }
    for ParsedPatch { key, var_index, patches, priority } in parsed {
        // Store variable class for nemesis variable to replace
        if let Some(var_index) = var_index {
            variable_class_map.0.entry(key.clone()).or_insert(var_index);
        }
        by_template.entry(key).or_default().files.push((patches, priority));
    }

    // 3/3: Resolve priorities per template in parallel.
    // Each template is owned by exactly one thread, so the inner maps are never contended.
    let borrowed_patches = by_template
        .into_par_iter()
        .map(|(key, TemplatePatches { base, files })| {
            for (patches, priority) in files {
                for (json_path, value) in patches {
                    let is_seq = matches!(
                        value.action,
                        json_patch::Action::Seq { .. } | json_patch::Action::SeqPush
                    );
                    let value = ValueWithPriority::new(value, priority);
                    if is_seq {
                        base.seq.insert(json_path, value);
                    } else {
                        base.one.insert(json_path, value); // Pure: no add and remove because of single value
                    }
                }
            }
            (key, base)
        })
        .collect();

    (
        PatchCollection {
            borrowed_patches: BehaviorPatchesMap(borrowed_patches),
            behavior_graph_data_map: variable_class_map,
        },
        errors,
    )
}

/// One parsed Nemesis patch file.
struct ParsedPatch<'a> {
    key: TemplateKey<'static>,
    /// Index(e.g. `#0000`) of `hkbBehaviorGraphData` used to replace Nemesis variables.
    var_index: Option<Cow<'static, str>>,
    patches: PatchesMap<'a>,
    priority: usize,
}

/// Patches targeting one template.
#[derive(Default)]
struct TemplatePatches<'a> {
    /// Already collected patches (e.g. FNIS)
    base: HkxPatchMaps<'a>,
    /// Parsed Nemesis patch files in input order.
    files: Vec<(PatchesMap<'a>, usize)>,
}

fn parse_one_patch<'a>(
    path: &Path,
    xml: &'a str,
    priority: usize,
    config: &Config,
) -> Result<ParsedPatch<'a>> {
    let (patches, parsed_var_index) = parse_nemesis_patch(xml, config.hack_options.map(Into::into))
        .with_context(|_| NemesisXmlErrSnafu { path })?;

    let nemesis_path = parse_nemesis_path(path)?;
    let key = nemesis_path
        .to_template_key()
        .with_context(|| FailedToCastNemesisPathToTemplateKeySnafu { path })?;

    let var_index = match nemesis_path.get_variable_index() {
        Some(master_behavior_graph_index) => Some(Cow::Borrowed(master_behavior_graph_index)),
        None => parsed_var_index.map(|index| Cow::Owned(index.to_string())),
    };

    Ok(ParsedPatch { key, var_index, patches, priority })
}
