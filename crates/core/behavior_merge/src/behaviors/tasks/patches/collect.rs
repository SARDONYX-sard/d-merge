use std::{borrow::Cow, path::Path};

use behavior_merge_types::config::HiddenRules;
use fast_hash::HashMap;
use json_patch::{JsonPatch, ValueWithPriority, json_path::JsonPath};
use nemesis_xml::patch::{PatchesMap, parse_nemesis_patch};
use rayon::{iter::Either, prelude::*};
use snafu::{OptionExt as _, ResultExt as _};

use super::{
    paths::{
        collect::{Category, collect_nemesis_paths},
        parse::parse_nemesis_path,
    },
    read_stats::{self, ModTimer, log_slowest_mods},
};
use crate::{
    Config, PriorityMap,
    behaviors::tasks::{
        patches::types::{
            BehaviorIndexes, BehaviorPatchesMap, HkxPatchMaps, OwnedPatchMap, OwnedPatches, PatchCollection,
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
pub(crate) fn collect_owned_patches(nemesis_entries: &PriorityMap, config: &Config) -> OwnedPatches {
    // NOTE: Walking and reading take seconds when the OS file cache is cold, so report the progress per mod dir.
    let reporter = StatusReportCounter::new(&config.status_report, ReportType::ReadingPatches, nemesis_entries.len());

    // Walk, read and sort into each category at once per mod dir. (No intermediate `Vec` of paths)
    // - No need to sort. Conflicts are resolved by priority, not by this order.
    // - For thousands of small files, blocking reads on rayon are much cheaper than
    //   spawning a tokio task (+ `spawn_blocking`) per file.
    #[cfg(feature = "tracing")]
    let start = std::time::Instant::now();

    let measure = read_stats::is_enabled();
    let (owned, mod_read_times) = nemesis_entries
        .par_iter()
        .map(|(mod_dir, &priority)| {
            let timer = measure.then(|| ModTimer::start(mod_dir));
            let patches = collect_nemesis_paths(Path::new(mod_dir))
                .fold(OwnedPatches::default, |mut acc, (category, path)| {
                    match read_stats::read_to_string(timer.as_ref(), &path) {
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
            (patches, timer.map(ModTimer::finish).into_iter().collect::<Vec<_>>())
        })
        .reduce(
            || (OwnedPatches::default(), Vec::new()),
            |(patches, mut times), (other_patches, other_times)| {
                times.extend(other_times);
                (patches.merge(other_patches), times)
            },
        );

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
    log_slowest_mods(mod_read_times);

    owned
}

pub(crate) fn collect_borrowed_patches<'a>(
    owned_patches: &'a OwnedPatchMap,
    config: &Config,
    fnis_patches: PatchCollection<'a>,
) -> (PatchCollection<'a>, Vec<Error>) {
    let PatchCollection { borrowed_patches: raw_borrowed_patches, behavior_index_map } = fnis_patches;

    let reporter = StatusReportCounter::new(&config.status_report, ReportType::ParsingPatches, owned_patches.len());

    // 1/3: Parse each file in parallel into thread-local data (no shared map, no lock).
    //
    // NOTE: `partition_map` into `Vec` keeps the input order, so the later steps are deterministic.
    let (parsed, errors): (Vec<ParsedPatch<'a, '_>>, Vec<Error>) = owned_patches
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
    let mut by_template: HashMap<TemplateKey<'static>, TemplatePatches<'a, '_>> = HashMap::default();
    for (key, maps) in raw_borrowed_patches.0 {
        by_template.entry(key).or_default().base = maps;
    }
    for ParsedPatch { key, indexes, patches, priority, hidden } in parsed {
        // Store indexes for nemesis variable to replace
        if let Some(indexes) = indexes {
            behavior_index_map.insert(key.clone(), indexes);
        }
        by_template.entry(key).or_default().files.push((patches, priority, hidden));
    }

    // 3/3: Resolve priorities per template in parallel.
    // Each template is owned by exactly one thread, so the inner maps are never contended.
    let borrowed_patches = by_template
        .into_par_iter()
        .map(|(key, TemplatePatches { base, files })| {
            for (patches, priority, hidden) in files {
                for (json_path, value) in patches {
                    let Some(hidden) = hidden else {
                        insert_patch(&base, json_path, value, priority);
                        continue;
                    };
                    for left in hidden.apply(&json_path, value) {
                        insert_patch(&base, json_path.clone(), left, priority);
                    }
                }
            }
            (key, base)
        })
        .collect();

    (PatchCollection { borrowed_patches: BehaviorPatchesMap(borrowed_patches), behavior_index_map }, errors)
}

fn insert_patch<'a>(base: &HkxPatchMaps<'a>, path: JsonPath<'a>, patch: JsonPatch<'a>, priority: usize) {
    let is_seq = matches!(patch.action, json_patch::Action::Seq { .. } | json_patch::Action::SeqPush);
    let patch = ValueWithPriority::new(patch, priority);
    if is_seq {
        base.seq.insert(path, patch);
    } else {
        base.one.insert(path, patch); // Pure: no add and remove because of single value
    }
}

/// The mod code in the path of a patch file.
///
/// e.g. `/some/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt` -> `flinch`
fn mod_code_of(path: &Path) -> Option<&str> {
    let parts: Vec<_> = path.iter().collect();
    let is_engine = |part: &&std::ffi::OsStr| {
        ["Nemesis_Engine", "Nemesis_EngineExt"].iter().any(|engine| part.eq_ignore_ascii_case(engine))
    };
    let engine = parts.iter().rposition(is_engine)?;
    parts.get(engine + 2)?.to_str()
}

/// One parsed Nemesis patch file.
struct ParsedPatch<'a, 'c> {
    key: TemplateKey<'static>,
    /// Indexes used to replace Nemesis variables.
    indexes: Option<BehaviorIndexes>,
    patches: PatchesMap<'a>,
    priority: usize,
    /// What is hidden of its mod in its template.
    hidden: Option<&'c HiddenRules>,
}

/// Patches targeting one template.
#[derive(Default)]
struct TemplatePatches<'a, 'c> {
    /// Already collected patches (e.g. FNIS)
    base: HkxPatchMaps<'a>,
    /// Parsed Nemesis patch files in input order.
    files: Vec<(PatchesMap<'a>, usize, Option<&'c HiddenRules>)>,
}

fn parse_one_patch<'a, 'c>(
    path: &Path,
    xml: &'a str,
    priority: usize,
    config: &'c Config,
) -> Result<ParsedPatch<'a, 'c>> {
    // NOTE: The root class index is returned only if it is `hkbBehaviorGraphStringData`.
    let (patches, parsed_string_data_index) =
        parse_nemesis_patch(xml, config.hack_options.map(Into::into)).with_context(|_| NemesisXmlErrSnafu { path })?;

    let nemesis_path = parse_nemesis_path(path)?;
    let key = nemesis_path.to_template_key().with_context(|| FailedToCastNemesisPathToTemplateKeySnafu { path })?;

    let indexes = match nemesis_path.get_master_index() {
        Some(index) => Some(BehaviorIndexes {
            string_data: Cow::Borrowed(index.master_string_data_index),
            behavior_graph: Some(Cow::Borrowed(index.master_behavior_graph_index)),
        }),
        // `hkbBehaviorGraphData` is unknown from the patch xml.
        None => parsed_string_data_index
            .map(|index| BehaviorIndexes { string_data: Cow::Owned(index.to_string()), behavior_graph: None }),
    };

    let hidden = if config.hidden_patches.is_empty() {
        None
    } else {
        let target = key.as_meshes_inner_path().to_string_lossy();
        mod_code_of(path).and_then(|code| config.hidden_patches.rules_of(code, &target))
    };

    Ok(ParsedPatch { key, indexes, patches, priority, hidden })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_mod_code_of_a_patch_file() {
        let code = |path: &str| mod_code_of(Path::new(path)).map(str::to_owned);
        assert_eq!(code("/a/Nemesis_Engine/mod/flinch/_1stperson/0_master/#0106.txt").as_deref(), Some("flinch"));
        assert_eq!(code("/a/nemesis_engineext/mod/para/meshes/x.bin/#1.txt").as_deref(), Some("para"));
        assert_eq!(code("/a/b/#1.txt"), None);
    }
}
