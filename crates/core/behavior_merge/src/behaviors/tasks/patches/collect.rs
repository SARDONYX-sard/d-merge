use std::{borrow::Cow, path::Path};

use fast_hash::HashMap;
use json_patch::ValueWithPriority;
use nemesis_xml::patch::{PatchesMap, parse_nemesis_patch};
use rayon::{iter::Either, prelude::*};
use snafu::{OptionExt as _, ResultExt as _};

use super::paths::{
    collect::{Category, collect_nemesis_paths},
    parse::parse_nemesis_path,
};
use crate::{
    Config, PriorityMap,
    behaviors::tasks::{
        patches::types::{
            BehaviorIndexes, BehaviorPatchesMap, HkxPatchMaps, OwnedPatchMap, OwnedPatches,
            PatchCollection,
        },
        templates::key::TemplateKey,
    },
    config::{ReportType, StatusReportCounter},
    errors::{Error, FailedToCastNemesisPathToTemplateKeySnafu, NemesisXmlErrSnafu, Result},
};

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
    // NOTE: Walking and reading take seconds when the OS file cache is cold, so report the progress per mod dir.
    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::ReadingPatches,
        nemesis_entries.len(),
    );

    // Walk, read and sort into each category at once per mod dir. (No intermediate `Vec` of paths)
    // - No need to sort. Conflicts are resolved by priority, not by this order.
    // - For thousands of small files, blocking reads on rayon are much cheaper than
    //   spawning a tokio task (+ `spawn_blocking`) per file.
    #[cfg(feature = "tracing")]
    let start = std::time::Instant::now();

    let owned = nemesis_entries
        .par_iter()
        .map(|(mod_dir, &priority)| {
            let patches = collect_nemesis_paths(Path::new(mod_dir))
                .fold(OwnedPatches::default, |mut acc, (category, path)| {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => match category {
                            Category::Nemesis => {
                                acc.owned_patches.insert(path, (content, priority));
                            }
                            Category::Adsf => {
                                acc.adsf_patches.insert(path, (content, priority));
                            }
                            Category::Asdsf => {
                                acc.asdsf_patches.insert(path, (content, priority));
                            }
                        },
                        Err(source) => acc.errors.push(Error::FailedIo { path, source }),
                    }
                    acc
                })
                .reduce(OwnedPatches::default, OwnedPatches::merge);
            reporter.increment();
            patches
        })
        .reduce(OwnedPatches::default, OwnedPatches::merge);

    // NOTE: With a cold OS file cache, the time is dominated by the fixed cost per file open
    // (e.g. ~25k files, 54MB: ~1ms/file, even on SSD), not by the bandwidth.
    //
    // NOTE: Log the counts to detect silently missing files. (e.g. a walker that drops entries on a busy thread pool)
    #[cfg(feature = "tracing")]
    tracing::info!(
        mod_dirs = nemesis_entries.len(),
        nemesis = owned.owned_patches.len(),
        adsf = owned.adsf_patches.0.len(),
        asdsf = owned.asdsf_patches.0.len(),
        errors = owned.errors.len(),
        elapsed = ?start.elapsed(),
        "Read Nemesis patch files"
    );

    owned
}

pub(crate) fn collect_borrowed_patches<'a>(
    owned_patches: &'a OwnedPatchMap,
    config: &Config,
    fnis_patches: PatchCollection<'a>,
) -> (PatchCollection<'a>, Vec<Error>) {
    let PatchCollection { borrowed_patches: raw_borrowed_patches, behavior_index_map } =
        fnis_patches;

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
    let mut by_template: HashMap<TemplateKey<'static>, TemplatePatches<'a>> = HashMap::default();
    for (key, maps) in raw_borrowed_patches.0 {
        by_template.entry(key).or_default().base = maps;
    }
    for ParsedPatch { key, indexes, patches, priority } in parsed {
        // Store indexes for nemesis variable to replace
        if let Some(indexes) = indexes {
            behavior_index_map.insert(key.clone(), indexes);
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
            behavior_index_map,
        },
        errors,
    )
}

/// One parsed Nemesis patch file.
struct ParsedPatch<'a> {
    key: TemplateKey<'static>,
    /// Indexes used to replace Nemesis variables.
    indexes: Option<BehaviorIndexes>,
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
    // NOTE: The root class index is returned only if it is `hkbBehaviorGraphStringData`.
    let (patches, parsed_string_data_index) =
        parse_nemesis_patch(xml, config.hack_options.map(Into::into))
            .with_context(|_| NemesisXmlErrSnafu { path })?;

    let nemesis_path = parse_nemesis_path(path)?;
    let key = nemesis_path
        .to_template_key()
        .with_context(|| FailedToCastNemesisPathToTemplateKeySnafu { path })?;

    let indexes = match nemesis_path.get_master_index() {
        Some(index) => Some(BehaviorIndexes {
            string_data: Cow::Borrowed(index.master_string_data_index),
            behavior_graph: Some(Cow::Borrowed(index.master_behavior_graph_index)),
        }),
        // `hkbBehaviorGraphData` is unknown from the patch xml.
        None => parsed_string_data_index.map(|index| BehaviorIndexes {
            string_data: Cow::Owned(index.to_string()),
            behavior_graph: None,
        }),
    };

    Ok(ParsedPatch { key, indexes, patches, priority })
}
