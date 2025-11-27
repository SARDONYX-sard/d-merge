mod alternative;
mod anim_var;
mod furniture;
mod gen_list_patch;
pub mod generated_behaviors;
mod global;
mod hkx_convert;
mod kill_move;
mod offset_arm;
mod pair;

use std::borrow::Cow;

use json_patch::{json_path, Action, JsonPatch, JsonPath, Op, ValueWithPriority};
use rayon::iter::Either;
use rayon::prelude::*;
use simd_json::json_typed;
use snafu::ResultExt;
use winnow::Parser;

use crate::behaviors::tasks::adsf::AdsfPatch;
use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::parse_fnis_list;
use crate::behaviors::tasks::fnis::patch_gen::gen_list_patch::{generate_patch, OneListPatch};
use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::{
    BehaviorEntry, DEFAULT_FEMALE, DRAUGR_SKELETON,
};
use crate::behaviors::tasks::fnis::patch_gen::global::_0_master::new_global_master_patch;
use crate::behaviors::tasks::fnis::patch_gen::global::mt_behavior::new_mt_global_patch;
use crate::behaviors::tasks::fnis::patch_gen::hkx_convert::AnimIoJob;
use crate::behaviors::tasks::patches::types::{
    BehaviorGraphDataMap, BehaviorPatchesMap, PatchCollection,
};
use crate::behaviors::tasks::templates::key::{
    THREAD_PERSON_0_MASTER_KEY, THREAD_PERSON_MT_BEHAVIOR_KEY,
};
use crate::config::{Config, ReportType, StatusReportCounter};
use crate::errors::{Error, FailedParseFnisModListSnafu};

pub use crate::behaviors::tasks::fnis::patch_gen::gen_list_patch::FnisPatchGenerationError;

pub(crate) type JsonPatchPairs<'a> = Vec<(JsonPath<'a>, ValueWithPriority<'a>)>;

struct ModProcessingResult<'a> {
    adsf_patches: Vec<AdsfPatch<'a>>,
    furniture_groups: Vec<String>,
    conversion_jobs: Vec<AnimIoJob>,
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
) -> (PatchCollection<'a>, Vec<AdsfPatch<'a>>, Vec<Error>) {
    let borrowed_patches = BehaviorPatchesMap::default();
    let behavior_graph_data_map = BehaviorGraphDataMap::new();

    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::GeneratingFnisPatches,
        mods_patches.len(),
    );

    let (successes, errors): (Vec<ModProcessingResult<'a>>, Vec<_>) = mods_patches
        .par_iter()
        .map(|owned_data| {
            let list = match parse_fnis_list
                .parse(&owned_data.list_content)
                .map_err(|e| serde_hkx::errors::readable::ReadableError::from_parse(e))
                .with_context(|_| FailedParseFnisModListSnafu {
                    path: owned_data.to_list_path(),
                }) {
                Ok(list) => list,
                Err(err) => {
                    reporter.increment();
                    return Either::Right(vec![err]);
                }
            };
            #[cfg(feature = "tracing")]
            tracing::debug!("{}: \n{list:#?}", owned_data.to_list_path().display());

            let OneListPatch {
                animation_paths: animations,
                events,
                adsf_patches,
                one_master_patches,
                seq_master_patches,
                one_mt_behavior_patches,
                seq_mt_behavior_patches,
                furniture_group_root_indexes,
                mut conversion_jobs,
            } = match generate_patch(owned_data, list, config) {
                Ok(patches) => patches,
                Err(err) => {
                    reporter.increment();
                    match err {
                        FnisPatchGenerationError::FailedToConvertAltAnimToOAR { errors } => {
                            return Either::Right(errors);
                        }
                        _ => return Either::Right(vec![Error::from(err)]),
                    }
                }
            };

            if owned_data.behavior_entry.is_3rd_person_character() {
                let entry = borrowed_patches
                    .0
                    .entry(THREAD_PERSON_MT_BEHAVIOR_KEY)
                    .or_default();

                for (path, patch) in one_mt_behavior_patches {
                    entry.one.insert(path, patch);
                }
                for (path, patch) in seq_mt_behavior_patches {
                    entry.seq.insert(path, patch);
                }
            }

            // Add patches to master.xml
            {
                let master_template_key =
                    owned_data.behavior_entry.to_master_behavior_template_key();

                // NOTE: By using `contains` instead of `.entry`, we avoid unnecessary cloning.
                if !behavior_graph_data_map.0.contains_key(&master_template_key) {
                    behavior_graph_data_map.0.insert(
                        master_template_key.clone(),
                        owned_data.behavior_entry.master_behavior_graph_index,
                    );
                }

                // Push Mod Root behavior to master xml
                let mut entry = borrowed_patches.0.entry(master_template_key).or_default();
                {
                    let (one_gen, one_state_info, seq_state) =
                        new_injectable_mod_root_behavior(owned_data);
                    entry.one.insert(one_gen.0, one_gen.1);
                    entry.one.insert(one_state_info.0, one_state_info.1);
                    entry.seq.insert(seq_state.0, seq_state.1);
                }

                // Insert patches for FNIS_*_List.txt
                // additions only, there won't be any duplicate keys, so we should be able to use `par_extend`.
                entry.one.0.par_extend(one_master_patches);
                for (path, patch) in seq_master_patches {
                    entry.seq.insert(path, patch);
                }

                if !events.is_empty() {
                    let mut events: Vec<_> = events.into_iter().collect();
                    events.par_sort_unstable();
                    let patches = new_push_events_seq_patch(
                        &events,
                        owned_data.behavior_entry.master_string_data_index,
                        owned_data.behavior_entry.master_behavior_graph_index,
                        owned_data.priority,
                    );
                    for (path, patch) in patches {
                        entry.seq.insert(path, patch);
                    }
                }
            }

            // Push One Mod animations

            if let Some(job) = hkx_convert::prepare_behavior_conversion_job(owned_data, config) {
                conversion_jobs.push(job);
            }
            if !animations.is_empty() {
                let mut animations: Vec<_> = animations.into_iter().collect();
                animations.par_sort_unstable(); // NOTE: The addition of animations has been tested to work in any order, but just to be safe.

                conversion_jobs.par_extend(hkx_convert::prepare_conversion_jobs(
                    &animations,
                    owned_data,
                    config,
                ));

                new_push_anim_seq_patch(
                    &animations,
                    owned_data,
                    owned_data.behavior_entry,
                    &borrowed_patches,
                );

                // NOTE: Since `events` shares the master file, there's no need to add it.
                match owned_data.behavior_entry.behavior_object {
                    // NOTE: The sync between `defaultmale` and `defaultfemale` must be performed.
                    "character" => {
                        new_push_anim_seq_patch(
                            &animations,
                            owned_data,
                            &DEFAULT_FEMALE,
                            &borrowed_patches,
                        );
                    }
                    // NOTE: Adding animation only to `draugr` will cause `draugrskeleton` to assume the A pose.
                    //       Therefore, we must add it in the same manner.
                    "draugr" => {
                        new_push_anim_seq_patch(
                            &animations,
                            owned_data,
                            &DRAUGR_SKELETON,
                            &borrowed_patches,
                        );
                    }
                    _ => {}
                }
            };

            reporter.increment();

            Either::Left(ModProcessingResult {
                adsf_patches,
                furniture_groups: furniture_group_root_indexes,
                conversion_jobs,
            })
        })
        .collect();

    let (adsf_patches, anim_groups_states, conversion_jobs) = successes
        .into_par_iter()
        .map(|res| (res.adsf_patches, res.furniture_groups, res.conversion_jobs))
        .reduce(
            || (Vec::new(), Vec::new(), Vec::new()),
            |(mut adsf1, mut furn1, mut jobs1), (adsf2, furn2, jobs2)| {
                adsf1.par_extend(adsf2);
                furn1.par_extend(furn2);
                jobs1.par_extend(jobs2);
                (adsf1, furn1, jobs1)
            },
        );

    let mut errors: Vec<Error> = errors.into_par_iter().flatten().collect();

    // FIXME?: Unknown causes errors due to mutexes in MO2
    rayon::scope(|s| {
        s.spawn(|_| {
            errors.par_extend(hkx_convert::run_conversion_jobs(
                conversion_jobs,
                config.output_target,
            ));
        });
    });

    // The inclusion of a patch for `0_master` implies that a class for FNIS options for `0_master` is also required.
    if borrowed_patches.0.contains_key(&THREAD_PERSON_0_MASTER_KEY) {
        borrowed_patches
            .0
            .entry(THREAD_PERSON_0_MASTER_KEY)
            .or_default()
            .one
            .0
            // Safety: This only adds private global indexes and does not conflict with the class_name indexes.
            .par_extend(new_global_master_patch(0));
    }

    if borrowed_patches
        .0
        .contains_key(&THREAD_PERSON_MT_BEHAVIOR_KEY)
    {
        let (one, seq) = new_mt_global_patch(anim_groups_states, 0);
        let mut entry = borrowed_patches
            .0
            .entry(THREAD_PERSON_MT_BEHAVIOR_KEY)
            .or_default();

        // Safety: This only adds private global indexes and does not conflict with the class_name indexes.
        entry.one.0.par_extend(one);
        for (path, patch) in seq {
            entry.seq.insert(path, patch);
        }
    }

    (
        PatchCollection {
            borrowed_patches,
            behavior_graph_data_map,
        },
        adsf_patches,
        errors,
    )
}

/// Register a mod's root behavior (`behaviors\FNIS_<namespace>_Behavior.hkx`)
/// into `meshes\actors\character\behaviors\0_master.xml`.
///
/// - `behavior_id`: Unique identifier for the behavior (e.g., `#<namespace>${index}`).
/// - `behavior_path`: Path to the behavior file used in `hkbBehaviorReferenceGenerator.behavior_name`.
fn new_injectable_mod_root_behavior<'a>(
    owned_data: &'a OwnedFnisInjection,
) -> (
    (JsonPath<'a>, ValueWithPriority<'a>),
    (JsonPath<'a>, ValueWithPriority<'a>),
    (JsonPath<'a>, ValueWithPriority<'a>),
) {
    // NOTE: To learn the additional method, I enabled only one FNIS mod and ran it, then read the XML in tools/*/`temporary_logs`.
    let namespace = owned_data.namespace.as_str();
    let priority = owned_data.priority;
    let behavior_path = owned_data.behavior_path.as_str();
    let new_generator_index = owned_data.next_class_name_attribute();
    let new_root_state_info_index = owned_data.next_class_name_attribute();
    let master_index = owned_data.behavior_entry.master_behavior_index;

    let one_state_info = (
        vec![
            Cow::Owned(new_root_state_info_index.clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                        "__ptr": new_root_state_info_index,
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": "#0000",
                        "exitNotifyEvents": "#0000",
                        "transitions": "#0000",
                        "generator": new_generator_index,
                        "name": format!("FNIS_State{priority}"),
                        "stateId": 1000 + priority, // FIXME?
                        "probability": 1.0,
                        "enable": true
                }),
            },
            priority,
        },
    );

    let one_gen = (
        vec![
            Cow::Owned(new_generator_index.clone()),
            Cow::Borrowed("hkbBehaviorReferenceGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": new_generator_index,
                    "variableBindingSet": "#0000", // null
                    "userData": 0,

                    // NOTE: FNIS_ROOT_BFR{index}: In FNIS, it's actually the ordering index.
                    // but here we use priority instead.
                    "name": format!("FNIS_ROOT_BFR_{namespace}_{priority}"), // StringPtr
                    "behaviorName": behavior_path, // StringPtr
                }),
            },
            priority,
        },
    );

    let seq_state = (
        json_path![master_index, "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::SeqPush,
                value: json_typed!(borrowed, [new_root_state_info_index]),
            },
            priority,
        },
    );

    (one_gen, one_state_info, seq_state)
}

/// Register event name & event flag(`"0"`).
pub fn new_push_events_seq_patch<'a>(
    events: &[Cow<'_, str>],
    string_data_index: &'static str,
    behavior_graph_index: &'static str,
    priority: usize,
) -> [(JsonPath<'a>, ValueWithPriority<'a>); 2] {
    [
        (
            json_path![
                string_data_index,
                "hkbBehaviorGraphStringData",
                "eventNames",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(borrowed, events),
                },
                priority,
            },
        ),
        (
            json_path![behavior_graph_index, "hkbBehaviorGraphData", "eventInfos"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(
                        borrowed,
                        events
                            .par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "flags": "0"
                                })
                            })
                            .collect::<Vec<_>>()
                    ),
                },
                priority,
            },
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

        let animations: Vec<String> = animations
            .par_iter()
            .map(|anim_file| format!("Animations\\{namespace}\\{anim_file}"))
            .collect();

        (
            json_path![index, "hkbCharacterStringData", "animationNames"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: json_typed!(borrowed, animations),
                },
                priority,
            },
        )
    };

    #[cfg(feature = "tracing")]
    tracing::debug!("FNIS Generated for animations: {json_path:?}: {patch:#?}");

    patches
        .0
        .entry(behavior_key)
        .or_default()
        .seq
        .insert(json_path, patch);
}
