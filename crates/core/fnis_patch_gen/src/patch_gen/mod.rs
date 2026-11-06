mod alternate;
mod anim_var;
mod chair;
mod classes;
mod dummy_esp;
mod furniture;
mod gen_list_patch;
pub(crate) mod generated_behaviors;
mod global;
mod io_jobs;
mod kill_move;
mod offset_arm;
mod pair;
mod skeleton_arm_fix;
#[cfg(test)]
pub(crate) mod snapshot_tests;

use std::borrow::Cow;

use fast_hash::HashSet;
use fnis_list::parse_fnis_list;
use json_patch::{Action, JsonPatch, JsonPath, Op, ValueWithPriority, json_path, json_value};
use rayon::{iter::Either, prelude::*};
use simd_json::BorrowedValue;
use snafu::ResultExt;
use winnow::Parser;

pub use self::io_jobs::FNISIoJobRunner;
use self::{
    classes::StateInfo,
    gen_list_patch::{OneListPatch, generate_patch},
    generated_behaviors::{BehaviorEntry, DEFAULT_FEMALE, DRAUGR_SKELETON},
    global::{
        _0_master::{alternate::new_push_alt_anim_values_seq_patch, new_global_master_patch},
        mt_behavior::new_mt_global_patch,
    },
    io_jobs::AnimIoJob,
    kill_move::calculate_hash,
};
use crate::{
    adsf::AdsfPatch,
    asdsf::AsdsfPatch,
    collect::owned::OwnedFnisInjection,
    config::{Config, ReportType, StatusReportCounter},
    errors::{Error, FailedParseFnisModListSnafu},
    patches::{BehaviorIndexMap, BehaviorIndexes, BehaviorPatchesMap, PatchCollection},
    template_key::{THREAD_PERSON_0_MASTER_KEY, THREAD_PERSON_MT_BEHAVIOR_KEY},
};

pub(crate) type JsonPatchPairs<'a> = Vec<(JsonPath<'a>, ValueWithPriority<'a>)>;

/// A patch that adds a new class to a template.
///
/// e.g. `new_class_patch("#Mod$1", "hkbStateMachine", 7, json_value!({ "__ptr": "#Mod$1" }))` is
/// `<hkobject name="#Mod$1" class="hkbStateMachine">` in the XML.
#[must_use]
pub(crate) fn new_class_patch<'a>(
    index: &'a str,
    class: &'static str,
    priority: usize,
    value: BorrowedValue<'a>,
) -> (JsonPath<'a>, ValueWithPriority<'a>) {
    let patch = JsonPatch { action: Action::Pure { op: Op::Add }, value };
    (vec![Cow::Borrowed(index), Cow::Borrowed(class)], ValueWithPriority { patch, priority })
}

/// A patch that appends to an array of a class: `value` is an array of what is appended.
///
/// e.g. `new_seq_push(json_path!["#0724", "hkbStateMachine", "states"], 7, json_value!(["#Mod$1"]))`
#[must_use]
pub(crate) const fn new_seq_push<'a>(
    path: JsonPath<'a>,
    priority: usize,
    value: BorrowedValue<'a>,
) -> (JsonPath<'a>, ValueWithPriority<'a>) {
    (path, ValueWithPriority { patch: JsonPatch { action: Action::SeqPush, value }, priority })
}

struct LocalAgg<'a> {
    borrowed_patches: BehaviorPatchesMap<'a>,
    behavior_index_map: BehaviorIndexMap<'a>,

    adsf_patches: Vec<AdsfPatch<'a>>,
    asdsf_patches: Vec<AsdsfPatch<'a>>,
    furniture_groups: Vec<&'a str>,

    conversion_jobs: Vec<AnimIoJob>,
}

impl<'a> LocalAgg<'a> {
    fn new() -> Self {
        Self {
            borrowed_patches: BehaviorPatchesMap::default(),
            behavior_index_map: BehaviorIndexMap::new(),
            adsf_patches: Vec::new(),
            asdsf_patches: Vec::new(),
            furniture_groups: Vec::new(),
            conversion_jobs: Vec::new(),
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.borrowed_patches.merge(other.borrowed_patches);

        self.behavior_index_map.0.par_extend(other.behavior_index_map.0);
        self.adsf_patches.par_extend(other.adsf_patches);
        self.asdsf_patches.par_extend(other.asdsf_patches);
        self.furniture_groups.par_extend(other.furniture_groups);
        self.conversion_jobs.par_extend(other.conversion_jobs);

        self
    }
}

/// Collect borrowed patches from multiple FNIS One Mods.
///
/// # Note
/// Since this function uses I/O internally for operations like hkx conversion,
/// do not call this function itself within `rayon::par_iter`.
/// This is because semaphore locks in MO2 may potentially deadlock.
pub fn collect_borrowed_patches<'a>(
    mods_patches: &'a [OwnedFnisInjection],
    config: &'a Config,
) -> (PatchCollection<'a>, Vec<AdsfPatch<'a>>, Vec<AsdsfPatch<'a>>, FNISIoJobRunner, Vec<Error>) {
    let reporter =
        StatusReportCounter::new(&config.status_report, ReportType::GeneratingFnisPatches, mods_patches.len());

    let (patches, mut errors): (Vec<_>, Vec<_>) = mods_patches.par_iter().partition_map(|owned_data| {
        match parse_fnis_list
            .parse(&owned_data.list_content)
            .map_err(|e| winnow_ext::ReadableError::from_parse(e))
            .with_context(|_| FailedParseFnisModListSnafu { path: owned_data.to_list_path() })
            .and_then(|list| {
                #[cfg(feature = "tracing")]
                tracing::debug!("{}: \n{list:#?}", owned_data.to_list_path().display());

                generate_patch(owned_data, list, config).map_err(|e| Error::FnisPatchGenerationError { source: e })
            }) {
            Ok(patches) => Either::Left((owned_data, patches)),
            Err(err) => {
                reporter.increment();
                Either::Right(err)
            }
        }
    });
    let LocalAgg {
        borrowed_patches,
        behavior_index_map,
        adsf_patches,
        asdsf_patches,
        furniture_groups,
        conversion_jobs,
    } = patches
        .into_par_iter()
        .fold(LocalAgg::new, |mut acc, (owned_data, patches)| {
            add_one_list(&mut acc, owned_data, patches, config);
            reporter.increment();
            acc
        })
        .reduce(LocalAgg::new, |a, b| a.merge(b));

    add_global_patches(&borrowed_patches, furniture_groups);

    let aa_base_map = apply_alternate_animations(&borrowed_patches, &conversion_jobs, config, &mut errors);
    #[cfg(feature = "tracing")]
    tracing::debug!("aa_base_map = {aa_base_map:#?}");

    if config.skeleton_arm_fix
        && let Err(e) = self::skeleton_arm_fix::apply(config, &borrowed_patches)
    {
        errors.push(Error::Custom { msg: e.to_string() });
    }

    if config.generate_fnis_esp
        && let Err(e) = self::dummy_esp::save_dummy_esp(&config.output_dir, config.output_target)
    {
        errors.push(Error::FNISGenerateEspError { source: e });
    };

    (
        PatchCollection { borrowed_patches, behavior_index_map },
        adsf_patches,
        asdsf_patches,
        FNISIoJobRunner::new(conversion_jobs, config.output_target, aa_base_map),
        errors,
    )
}

/// Adds what was generated from one `FNIS_*_List.txt`.
fn add_one_list<'a>(
    acc: &mut LocalAgg<'a>,
    owned_data: &'a OwnedFnisInjection,
    patches: OneListPatch<'a>,
    config: &'a Config,
) {
    let OneListPatch {
        animation_paths,
        events,
        adsf_patches,
        asdsf_patches,
        one_master_patches,
        seq_master_patches,
        one_mt_behavior_patches,
        seq_mt_behavior_patches,
        furniture_group_root_indexes,
        mut conversion_jobs,
    } = patches;

    if owned_data.behavior_entry.is_3rd_person_character() {
        add_mt_behavior_patches(acc, one_mt_behavior_patches, seq_mt_behavior_patches);
    }
    add_master_patches(acc, owned_data, one_master_patches, seq_master_patches, events);

    if let Some(job) = io_jobs::hkx::prepare_behavior_conversion_job(owned_data, config) {
        conversion_jobs.push(job);
    }
    if !animation_paths.is_empty() {
        let mut animations: Vec<_> = animation_paths.into_iter().collect();
        animations.par_sort_unstable(); // NOTE: The addition of animations has been tested to work in any order, but just to be safe.

        conversion_jobs.par_extend(io_jobs::hkx::prepare_conversion_jobs(&animations, owned_data, config));
        add_animation_paths(&animations, owned_data, &acc.borrowed_patches);
    }

    acc.adsf_patches.extend(adsf_patches);
    acc.asdsf_patches.extend(asdsf_patches);
    acc.furniture_groups.extend(furniture_group_root_indexes);
    acc.conversion_jobs.extend(conversion_jobs);
}

/// Adds patches to `mt_behavior.xml`.
fn add_mt_behavior_patches<'a>(acc: &LocalAgg<'a>, one: JsonPatchPairs<'a>, seq: JsonPatchPairs<'a>) {
    let entry = acc.borrowed_patches.0.entry(THREAD_PERSON_MT_BEHAVIOR_KEY).or_default();

    // Indexes of `meshes/actors/character/behaviors/mt_behavior.bin`
    if !acc.behavior_index_map.0.contains_key(&THREAD_PERSON_MT_BEHAVIOR_KEY) {
        acc.behavior_index_map.0.insert(
            THREAD_PERSON_MT_BEHAVIOR_KEY,
            BehaviorIndexes { string_data: Cow::Borrowed("#0083"), behavior_graph: Some(Cow::Borrowed("#0085")) },
        );
    }

    for (path, patch) in one {
        entry.one.insert(path, patch);
    }
    for (path, patch) in seq {
        entry.seq.insert(path, patch);
    }
}

/// Adds patches to the master file of the behavior, e.g. `0_master.xml`.
fn add_master_patches<'a>(
    acc: &LocalAgg<'a>,
    owned_data: &'a OwnedFnisInjection,
    one: JsonPatchPairs<'a>,
    seq: JsonPatchPairs<'a>,
    events: HashSet<Cow<'a, str>>,
) {
    let entry_of_behavior = owned_data.behavior_entry;
    let master_template_key = entry_of_behavior.to_master_behavior_template_key();

    // NOTE: By using `contains` instead of `.entry`, we avoid unnecessary cloning.
    if !acc.behavior_index_map.0.contains_key(&master_template_key) {
        acc.behavior_index_map.0.insert(
            master_template_key.clone(),
            BehaviorIndexes {
                string_data: Cow::Borrowed(entry_of_behavior.master_string_data_index),
                behavior_graph: Some(Cow::Borrowed(entry_of_behavior.master_behavior_graph_index)),
            },
        );
    }

    // Push Mod Root behavior to master xml
    let mut entry = acc.borrowed_patches.0.entry(master_template_key).or_default();
    let (one_gen, one_state_info, seq_state) = new_injectable_mod_root_behavior(owned_data);
    entry.one.insert(one_gen.0, one_gen.1);
    entry.one.insert(one_state_info.0, one_state_info.1);
    entry.seq.insert(seq_state.0, seq_state.1);

    // Insert patches for FNIS_*_List.txt
    // additions only, there won't be any duplicate keys, so we should be able to use `par_extend`.
    entry.one.par_extend(one);
    for (path, patch) in seq {
        entry.seq.insert(path, patch);
    }

    if !events.is_empty() {
        let mut events: Vec<_> = events.into_iter().collect();
        events.par_sort_unstable();
        let patches = new_push_events_seq_patch(
            events,
            entry_of_behavior.master_string_data_index,
            entry_of_behavior.master_behavior_graph_index,
            owned_data.priority,
        );
        for (path, patch) in patches {
            entry.seq.insert(path, patch);
        }
    }
}

/// Registers the animations of a mod in each template that has to know them.
fn add_animation_paths<'a>(
    animations: &[&'a str],
    owned_data: &'a OwnedFnisInjection,
    patches: &BehaviorPatchesMap<'a>,
) {
    new_push_anim_seq_patch(animations, owned_data, owned_data.behavior_entry, patches);

    // NOTE: Since `events` shares the master file, there's no need to add it.
    match owned_data.behavior_entry.behavior_object {
        // NOTE: The sync between `defaultmale` and `defaultfemale` must be performed.
        "character" => new_push_anim_seq_patch(animations, owned_data, &DEFAULT_FEMALE, patches),
        // NOTE: Adding animation only to `draugr` will cause `draugrskeleton` to assume the A pose.
        //       Therefore, we must add it in the same manner.
        "draugr" => new_push_anim_seq_patch(animations, owned_data, &DRAUGR_SKELETON, patches),
        _ => {}
    }
}

/// Adds the classes that all mods of a template share, once.
fn add_global_patches<'a>(patches: &BehaviorPatchesMap<'a>, furniture_groups: Vec<&'a str>) {
    // The inclusion of a patch for `0_master` implies that a class for FNIS options for `0_master` is also required.
    if patches.0.contains_key(&THREAD_PERSON_0_MASTER_KEY) {
        patches
            .0
            .entry(THREAD_PERSON_0_MASTER_KEY)
            .or_default()
            // Safety: This only adds private global indexes and does not conflict with the class_name indexes.
            .one
            .par_extend(new_global_master_patch(0));
    }

    if patches.0.contains_key(&THREAD_PERSON_MT_BEHAVIOR_KEY) {
        let (one, seq) = new_mt_global_patch(furniture_groups, 0);
        let mut entry = patches.0.entry(THREAD_PERSON_MT_BEHAVIOR_KEY).or_default();

        // Safety: This only adds private global indexes and does not conflict with the class_name indexes.
        entry.one.par_extend(one);
        for (path, patch) in seq {
            entry.seq.insert(path, patch);
        }
    }
}

/// If a mod has alternate animations: writes `fnis_aa/config.json`, and adds what `0_master` needs for
/// them. Returns the base of each group, for writing the configs of the slots later.
fn apply_alternate_animations(
    patches: &BehaviorPatchesMap<'_>,
    conversion_jobs: &[AnimIoJob],
    config: &Config,
    errors: &mut Vec<Error>,
) -> Option<alternate::aa_config::BaseMap> {
    if !conversion_jobs.par_iter().any(|job| matches!(job, AnimIoJob::FnisAANamespaceConfig(_))) {
        return None;
    }
    match alternate::aa_config::build_aa_config_from_jobs(conversion_jobs, &config.output_dir) {
        Ok(aa_base_map) => {
            for (path, patch) in new_push_alt_anim_values_seq_patch(0) {
                patches.0.entry(THREAD_PERSON_0_MASTER_KEY).or_default().seq.insert(path, patch);
            }
            Some(aa_base_map)
        }
        Err(e) => {
            errors.push(e);
            None
        }
    }
}

/// Register a mod's root behavior (`behaviors\FNIS_<namespace>_Behavior.hkx`)
/// into `meshes\actors\character\behaviors\0_master.xml`.
///
/// - `behavior_id`: Unique identifier for the behavior (e.g., `#<namespace>${index}`).
/// - `behavior_path`: Path to the behavior file used in `hkbBehaviorReferenceGenerator.behavior_name`.
fn new_injectable_mod_root_behavior<'a>(
    owned_data: &'a OwnedFnisInjection,
) -> ((JsonPath<'a>, ValueWithPriority<'a>), (JsonPath<'a>, ValueWithPriority<'a>), (JsonPath<'a>, ValueWithPriority<'a>))
{
    // NOTE: To learn the additional method, I enabled only one FNIS mod and ran it, then read the XML in tools/*/`temporary_logs`.
    let namespace = owned_data.namespace.as_str();
    let priority = owned_data.priority;
    let behavior_path = owned_data.behavior_path.as_str();
    let new_generator_index = owned_data.next_class_name_attribute();
    let new_root_state_info_index = owned_data.next_class_name_attribute();
    let master_index = owned_data.behavior_entry.master_behavior_index;

    let root_state_name = format!("FNIS_State{new_root_state_info_index}");

    let one_state_info = StateInfo {
        index: new_root_state_info_index,
        generator: new_generator_index,
        state_id: calculate_hash(root_state_name.as_str()), // FNIS: 1000 + n
        name: root_state_name.into(),
        ..Default::default()
    }
    .into_patch(priority);

    let one_gen = new_class_patch(
        new_generator_index,
        "hkbBehaviorReferenceGenerator",
        priority,
        json_value!({
            "__ptr": new_generator_index,
            "variableBindingSet": "#0000", // null
            "userData": 0,

            // NOTE: FNIS_ROOT_BFR{index}: In FNIS, it's actually the ordering index.
            // but here we use priority instead.
            "name": format!("FNIS_ROOT_BFR_{namespace}_{priority}"), // StringPtr
            "behaviorName": behavior_path, // StringPtr
        }),
    );

    let seq_state = new_seq_push(
        json_path![master_index, "hkbStateMachine", "states"],
        priority,
        json_value!([new_root_state_info_index]),
    );

    (one_gen, one_state_info, seq_state)
}

/// Register event name & event flag(`"0"`).
pub(crate) fn new_push_events_seq_patch<'a>(
    events: Vec<Cow<'a, str>>,
    string_data_index: &'static str,
    behavior_graph_index: &'static str,
    priority: usize,
) -> [(JsonPath<'a>, ValueWithPriority<'a>); 2] {
    let event_infos: Vec<_> = events.iter().map(|_| json_value!({ "flags": "0" })).collect();
    [
        new_seq_push(
            json_path![string_data_index, "hkbBehaviorGraphStringData", "eventNames"],
            priority,
            json_value!(events),
        ),
        new_seq_push(
            json_path![behavior_graph_index, "hkbBehaviorGraphData", "eventInfos"],
            priority,
            json_value!(event_infos),
        ),
    ]
}

/// Insert a new animation sequence patch into the borrowed patches map.
///
/// Create an additional patch for the animations for one of the following template files.
/// - `#0029`
///    - `meshes/actors/character/_1stperson/firstperson.xml`
///    - `meshes/actors/character/default_female/defaultfemale.xml`
///    - `meshes/actors/character/defaultmale/defaultmale.xml`
///
/// # Note
/// `animations`: Windows path to the Animations dir containing files within `meshes/actors/character/animations/<FNIS one mod namespace>/*.hkx`.
///
/// - sample animations
/// ```txt
/// [
///     "Animations\<FNIS one mod namespace>\sample.hkx",
///     "Animations\<FNIS one mod namespace>\sample1.hkx"
/// ]
/// ```
fn new_push_anim_seq_patch<'a>(
    animations: &[&'a str],
    owned_data: &'a OwnedFnisInjection,
    behavior_entry: &'static BehaviorEntry,
    patches: &BehaviorPatchesMap<'a>,
) {
    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let behavior_key = behavior_entry.to_default_behavior_template_key();

    let (json_path, patch) = {
        let index = behavior_entry.default_behavior_index;

        let animations: Vec<String> =
            animations.par_iter().map(|anim_file| format!("Animations\\{namespace}\\{anim_file}")).collect();

        new_seq_push(json_path![index, "hkbCharacterStringData", "animationNames"], priority, json_value!(animations))
    };

    #[cfg(feature = "tracing")]
    tracing::debug!("FNIS Generated for animations: {json_path:?}: {patch:#?}");

    patches.0.entry(behavior_key).or_default().seq.insert(json_path, patch);
}
