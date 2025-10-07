mod gen_list_patch;
pub mod generated_behaviors;
mod kill_move;
mod pair;

use std::borrow::Cow;
use std::path::PathBuf;

use dashmap::DashSet;
use json_patch::{json_path, JsonPatch, JsonPath, Op, OpRangeKind, ValueWithPriority};
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
use crate::behaviors::tasks::patches::types::{
    BehaviorStringDataMap, BorrowedPatches, RawBorrowedPatches,
};
use crate::behaviors::tasks::templates::key::{TemplateKey, THREAD_PERSON_0_MASTER_KEY};
use crate::config::{ReportType, StatusReportCounter, StatusReporterFn};
use crate::errors::{Error, FailedParseFnisModListSnafu};

pub use crate::behaviors::tasks::fnis::patch_gen::gen_list_patch::FnisPatchGenerationError;

/// For Seq patch
pub(crate) const PUSH_OP: OpRangeKind = OpRangeKind::Seq(json_patch::OpRange {
    op: Op::Add,
    range: 9998..9999,
});

pub(crate) type JsonPatchPairs<'a> = Vec<(JsonPath<'a>, ValueWithPriority<'a>)>;

pub fn collect_borrowed_patches<'a>(
    mods_patches: &'a [OwnedFnisInjection],
    status_reporter: &'a StatusReporterFn,
) -> (BorrowedPatches<'a>, Vec<AdsfPatch<'a>>, Vec<Error>) {
    let raw_borrowed_patches = RawBorrowedPatches::default();
    let template_keys = DashSet::new();
    let variable_class_map = BehaviorStringDataMap::new(); // TODO: Change to compile time phf map.

    let reporter = StatusReportCounter::new(
        status_reporter,
        ReportType::GeneratingFnisPatches,
        mods_patches.len(),
    );

    let (adsf_patches, errors): (Vec<_>, Vec<_>) = mods_patches
        .par_iter()
        .map(|owned_data| {
            reporter.increment();

            let list = match parse_fnis_list
                .parse(&owned_data.list_content)
                .map_err(|e| serde_hkx::errors::readable::ReadableError::from_parse(e))
                .with_context(|_| FailedParseFnisModListSnafu {
                    path: PathBuf::from(owned_data.to_list_path()),
                }) {
                Ok(list) => list,
                Err(err) => return Either::Right(err),
            };
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "{list_path}: \n{list:#?}",
                list_path = owned_data.to_list_path(),
            );

            let OneListPatch {
                animation_paths: animations,
                events,
                adsf_patches,
                one_master_patches,
                seq_master_patches,
            } = match generate_patch(owned_data, list) {
                Ok(patches) => patches,
                Err(err) => return Either::Right(Error::from(err)),
            };

            // Add patches to master.xml
            {
                let master_template_key =
                    owned_data.behavior_entry.to_master_behavior_template_key();

                // NOTE: By using `contains` instead of `.entry`, we avoid unnecessary cloning.
                if !template_keys.contains(&master_template_key) {
                    template_keys.insert(master_template_key.clone());
                };
                if !variable_class_map.0.contains_key(&master_template_key) {
                    variable_class_map.0.insert(
                        master_template_key.clone(),
                        owned_data.behavior_entry.master_string_data_index,
                    );
                }

                // Push Mod Root behavior to master xml
                let entry = raw_borrowed_patches
                    .0
                    .entry(master_template_key)
                    .or_default();
                {
                    let (one_gen, one_state_info, seq_state) =
                        new_injectable_mod_root_behavior(owned_data);
                    entry.one.insert(one_gen.0, one_gen.1);
                    entry.one.insert(one_state_info.0, one_state_info.1);
                    entry.seq.insert(seq_state.0, seq_state.1);
                }

                // Insert patches for FNIS_*_List.txt
                // TODO: If it's strictly for additions only, there won't be any duplicate keys, so we should be able to use `par_extend`.
                // entry.one.par_extend(one_master_patches);
                for (path, patch) in one_master_patches {
                    entry.one.insert(path, patch);
                }
                for (path, patch) in seq_master_patches {
                    entry.seq.insert(path, patch);
                }

                // TODO: Existing events must avoid duplicate additions. To achieve this, a table must be created. - FNIS_base, 0_master, mt_behavior
                // However, creating a table for each creature is impractical; in reality, it would likely be limited to pairAndKillMoves(character).
                if !events.is_empty() {
                    let mut events: Vec<_> = events.into_iter().collect(); // We'll probably end up using `.filter` and `phf_set!` to check here.
                    events.par_sort_unstable();
                    let patches = new_push_events_seq_patch(
                        &events,
                        owned_data.behavior_entry,
                        owned_data.priority,
                    );
                    for (path, patch) in patches {
                        entry.seq.insert(path, patch);
                    }
                }
            }

            // Push One Mod animations
            if !animations.is_empty() {
                let mut animations: Vec<_> = animations.into_iter().collect();
                animations.par_sort_unstable(); // NOTE: The addition of animations has been tested to work in any order, but just to be safe.

                new_push_anim_seq_patch(
                    &animations,
                    owned_data.behavior_entry,
                    owned_data.priority,
                    &raw_borrowed_patches,
                    &template_keys,
                );

                match owned_data.behavior_entry.behavior_object {
                    // NOTE: The sync between `defaultmale` and `defaultfemale` must be performed.
                    "character" => {
                        new_push_anim_seq_patch(
                            &animations,
                            &DEFAULT_FEMALE,
                            owned_data.priority,
                            &raw_borrowed_patches,
                            &template_keys,
                        );
                    }
                    // NOTE: Adding animation only to `draugr` will cause `dragurskeleton` to assume the A pose.
                    //       Therefore, we must add it in the same manner.
                    "draugr" => {
                        new_push_anim_seq_patch(
                            &animations,
                            &DRAUGR_SKELETON,
                            owned_data.priority,
                            &raw_borrowed_patches,
                            &template_keys,
                        );
                    }
                    _ => {}
                }
            };

            Either::Left(adsf_patches)
        })
        .collect();

    let adsf_patches: Vec<_> = adsf_patches.into_par_iter().flatten().collect();

    // The inclusion of a patch for `0_master` implies that a class for FNIS options for `0_master` is also required.
    if template_keys.contains(&THREAD_PERSON_0_MASTER_KEY) {
        raw_borrowed_patches
            .0
            .entry(THREAD_PERSON_0_MASTER_KEY)
            .or_default()
            .one
            .0
            // Safety: This only adds a private global index and does not conflict with the class_name index.
            .par_extend(new_global_alt_flags(0));
    };

    (
        BorrowedPatches {
            template_keys,
            borrowed_patches: raw_borrowed_patches,
            behavior_string_data_map: variable_class_map,
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
                op: OpRangeKind::Pure(Op::Add),
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
                op: OpRangeKind::Pure(Op::Add),
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
                op: PUSH_OP,
                value: json_typed!(borrowed, [new_root_state_info_index]),
            },
            priority,
        },
    );

    (one_gen, one_state_info, seq_state)
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
    animations: &[String],
    behavior_entry: &BehaviorEntry,
    priority: usize,
    raw_borrowed_patches: &RawBorrowedPatches<'a>,
    template_keys: &DashSet<TemplateKey<'static>>,
) {
    let behavior_key = behavior_entry.to_default_behavior_template_key();

    template_keys.insert(behavior_key.clone());

    let (json_path, patch) = {
        let index = behavior_entry.default_behavior_index;
        (
            json_path![index, "hkbCharacterStringData", "animationNames"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: PUSH_OP,
                    value: json_typed!(borrowed, animations),
                },
                priority,
            },
        )
    };

    #[cfg(feature = "tracing")]
    tracing::debug!("FNIS Generated for animations: {json_path:?}: {patch:#?}");

    raw_borrowed_patches
        .0
        .entry(behavior_key)
        .or_default()
        .seq
        .insert(json_path, patch);
}

fn new_push_events_seq_patch<'a>(
    events: &[&'a str],
    behavior_entry: &BehaviorEntry,
    priority: usize,
) -> [(JsonPath<'a>, ValueWithPriority<'a>); 2] {
    [
        (
            json_path![
                behavior_entry.master_string_data_index,
                "hkbBehaviorGraphStringData",
                "eventNames",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: PUSH_OP,
                    value: simd_json::json_typed!(borrowed, events),
                },
                priority,
            },
        ),
        (
            json_path![
                behavior_entry.master_behavior_graph_index,
                "hkbBehaviorGraphStringData",
                "eventInfos",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: PUSH_OP,
                    value: simd_json::json_typed!(
                        borrowed,
                        events
                            .par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "flags": 0
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

/// FNIS XML(name="#2526") - `HeadTrackingOff`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2526: &str = "FNIS_aa_global_auto_gen2526";

/// FNIS XML(name="#2527") - `HeadTrackingOn`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2527: &str = "FNIS_aa_global_auto_gen2527";

/// FNIS XML(name="#2528") - `AnimObjectUnequip`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2528: &str = "FNIS_aa_global_auto_gen2528";

/// FNIS XML(name="#2529") - `Multi (HeadTrackingOn + AnimObjectUnequip)`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2529: &str = "FNIS_aa_global_auto_gen2529";

/// FNIS XML(name="#2530") - `StartAnimatedCamera`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2530: &str = "FNIS_aa_global_auto_gen2530";

/// FNIS XML(name="#2531") - `StringEventPayload (Camera3rd [Cam3])`
pub(crate) const FNIS_AA_STRING_PAYLOAD_2531: &str = "FNIS_aa_global_auto_gen2531";

/// FNIS XML(name="#2532") - `EndAnimatedCamera`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2532: &str = "FNIS_aa_global_auto_gen2532";

/// FNIS XML(name="#2533") - `PairedKillTarget`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2533: &str = "FNIS_aa_global_auto_gen2533";

/// FNIS XML(name="#2534") - `Multi (StartAnimatedCamera + PairedKillTarget)`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2534: &str = "FNIS_aa_global_auto_gen2534";

/// Generate the Havok class corresponding to the options flags in FNIS_*_List.txt.
///
/// # Note
/// The classes generated here are used in `character/behaviors/0_master.xml`
/// and are generated for alternative animations(FNIS_aa).
///
/// However, they are actually also reused in PairedAndKillMoves, so they must be generated.
///
/// See: `FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\0_master_TEMPLATE.txt`
fn new_global_alt_flags<'a>(priority: usize) -> JsonPatchPairs<'a> {
    // single event (#2526, #2527, #2528, #2530, #2532, #2533)
    let single_events: [(&'static str, i32, Option<&'static str>); 6] = [
        (FNIS_AA_GLOBAL_AUTO_GEN_2526, 366, None), // HeadTrackingOff
        (FNIS_AA_GLOBAL_AUTO_GEN_2527, 367, None), // HeadTrackingOn
        (FNIS_AA_GLOBAL_AUTO_GEN_2528, 543, None), // AnimObjectUnequip
        (FNIS_AA_GLOBAL_AUTO_GEN_2530, 1061, Some("#2531")), // StartAnimatedCamera
        (FNIS_AA_GLOBAL_AUTO_GEN_2532, 1062, None), // EndAnimatedCamera
        (FNIS_AA_GLOBAL_AUTO_GEN_2533, 915, None), // PairedKillTarget
    ];

    let mut patches: JsonPatchPairs<'a> = single_events
        .par_iter()
        .map(|&(class_index, id, payload)| {
            (
                json_path![class_index, "hkbStateMachineEventPropertyArray"],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Pure(Op::Add),
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": class_index,
                            "events": [
                                {
                                    "id": id,
                                    "payload": payload.unwrap_or("null"),
                                }
                            ]
                        }),
                    },
                    priority,
                },
            )
        })
        .collect();

    // multi events #2529
    patches.push((
        json_path![
            FNIS_AA_GLOBAL_AUTO_GEN_2529,
            "hkbStateMachineEventPropertyArray"
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_GLOBAL_AUTO_GEN_2529,
                    "events": [
                        { "id": 367, "payload": "null" }, // HeadTrackingOn
                        { "id": 543, "payload": "null" }, // AnimObjectUnequip
                    ]
                }),
            },
            priority,
        },
    ));

    // multi events #2534
    patches.push((
        json_path![
            FNIS_AA_GLOBAL_AUTO_GEN_2534,
            "hkbStateMachineEventPropertyArray"
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_GLOBAL_AUTO_GEN_2534,
                    "events": [
                        { "id": 1061, "payload": FNIS_AA_STRING_PAYLOAD_2531 }, // StartAnimatedCamera
                        { "id": 915,  "payload": "null"   }, // PairedKillTarget
                    ]
                }),
            },
            priority,
        },
    ));

    // StringEventPayload #2531
    patches.push((
        json_path![FNIS_AA_STRING_PAYLOAD_2531, "hkbStringEventPayload"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_STRING_PAYLOAD_2531,
                    "data": "Camera3rd [Cam3]"
                }),
            },
            priority,
        },
    ));

    patches
}
