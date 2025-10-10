//! NOTE: To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\0_master_TEMPLATE.txt"
use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::{
    combinator::{flags::FNISAnimFlags, Trigger},
    patterns::pair_and_kill::{AnimObject, FNISPairedAndKillAnimation},
};
use crate::behaviors::tasks::fnis::patch_gen::global::patch_0_master::{
    FNIS_AA_GLOBAL_AUTO_GEN_2526, FNIS_AA_GLOBAL_AUTO_GEN_2527, FNIS_AA_GLOBAL_AUTO_GEN_2528,
    FNIS_AA_GLOBAL_AUTO_GEN_2529, FNIS_AA_GLOBAL_AUTO_GEN_2530, FNIS_AA_GLOBAL_AUTO_GEN_2532,
    FNIS_AA_GLOBAL_AUTO_GEN_2533, FNIS_AA_GLOBAL_AUTO_GEN_2534,
};
use crate::behaviors::tasks::fnis::patch_gen::{JsonPatchPairs, PUSH_OP};

/// Into `meshes\actors\character\behaviors\0_master.xml`.
pub fn new_kill_patches<'a>(
    paired_and_kill_animation: FNISPairedAndKillAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    // new C++ Havok class XML name attributes
    let class_indexes: [String; 26] =
        core::array::from_fn(|_| owned_data.next_class_name_attribute());
    let priority = owned_data.priority;
    let flags = paired_and_kill_animation.flag_set.flags;
    let player_event = paired_and_kill_animation.anim_event;
    let npc_event = format!("pa_{player_event}");
    let duration = paired_and_kill_animation.flag_set.duration;
    let anim_file = format!(
        "Animations\\{}\\{}",
        &owned_data.namespace, paired_and_kill_animation.anim_file
    ); // Animations\\$Fkm$

    let player_root_state_name = format!("Player_FNISkm{priority}"); // NOTE: must be unique in 0_master.xml
    let npc_root_state_name = format!("NPC_FNISkm{priority}"); // NOTE: must be unique in 0_master.xml

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.push(new_push_transitions_seq_patch(
        "#0789",
        [player_event, npc_event.as_str()],
        [&player_root_state_name, &npc_root_state_name],
        priority,
    ));
    // Push and register the Root `hkbStateMachineStateInfo` for both Player and NPC.
    seq_patches.push((
        json_path!["#0788", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, [class_indexes[0], class_indexes[12]]),
            },
            priority,
        },
    ));

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let class_index_to_anim_object_map = dashmap::DashMap::new();
    one_patches.par_extend(
        paired_and_kill_animation
            .anim_objects
            .par_iter()
            .enumerate()
            .map(|(index, AnimObject { name, role: _ })| {
                let new_anim_object_index = owned_data.next_class_name_attribute();
                class_index_to_anim_object_map.insert(index, new_anim_object_index.clone());

                let one_anim_obj = (
                    vec![
                        Cow::Owned(new_anim_object_index.clone()),
                        Cow::Borrowed("hkbStringEventPayload"),
                    ],
                    ValueWithPriority {
                        patch: JsonPatch {
                            op: OpRangeKind::Pure(Op::Add),
                            value: simd_json::json_typed!(borrowed, {
                                "__ptr": new_anim_object_index,
                                "data": name, // StringPtr
                            }),
                        },
                        priority,
                    },
                );
                one_anim_obj
            }),
    );

    one_patches.push(make_player_root_state_info_patch(
        &class_indexes[0],
        &class_indexes[1],
        flags,
        priority,
        player_root_state_name,
    ));

    one_patches.push((
        vec![
            Cow::Owned(class_indexes[1].clone()),
            Cow::Borrowed("hkbModifierGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[1],
                    "variableBindingSet": "#0000", // null
                    "userData": 1,
                    "name": format!("Player_FNISkm{priority}_ModGen"), // StringPtr
                    "modifier": class_indexes[14], // StringPtr
                    "generator": class_indexes[2], // StringPtr
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[2].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[2],
                    "variableBindingSet": class_indexes[3],
                    "userData": 0,
                    "name": format!("Player_FNISkm{priority}$_Behavior"), // FIXME? $1/1$
                    "eventToSendWhenStateOrTransitionChanges": {
                        "id": -1,
                        "payload": "#0000"
                    },
                    "startStateChooser": "#0000",
                    "startStateId": 0,
                    "returnToPreviousStateEventId": -1,
                    "randomTransitionEventId": -1,
                    "transitionToNextHigherStateEventId": -1,
                    "transitionToNextLowerStateEventId": -1,
                    "syncVariableIndex": -1,
                    "wrapAroundStateId": false,
                    "maxSimultaneousTransitions": 32,
                    "startStateMode": "START_STATE_MODE_DEFAULT",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                    "states": [class_indexes[4]],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[3].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                        "__ptr": class_indexes[3],
                        "bindings": [
                            {
                                "memberPath": "isActive",
                                "variableIndex": 51,
                                "bitIndex": -1,
                                "bindingType": "BINDING_TYPE_VARIABLE"
                            }
                        ],
                        "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[4].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[4],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[5],
                    "name": format!("Player_FNISkm{priority}_DisablePitch"),
                    "stateId": 0,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[5].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[5],
                    "variableBindingSet": &class_indexes[6],
                    "userData": 0,
                    "name": format!("Player_FNISkm{priority}_DisablePitch_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": {
                        "id": -1,
                        "payload": "#0000"
                    },
                    "startStateChooser": "#0000",
                    "startStateId": 0,
                    "returnToPreviousStateEventId": -1,
                    "randomTransitionEventId": -1,
                    "transitionToNextHigherStateEventId": -1,
                    "transitionToNextLowerStateEventId": -1,
                    "syncVariableIndex": -1,
                    "wrapAroundStateId": false,
                    "maxSimultaneousTransitions": 32,
                    "startStateMode": "START_STATE_MODE_DEFAULT",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                    "states": [&class_indexes[7]],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[6].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[6],
                    "bindings": [
                        {
                            "memberPath": "isActive",
                            "variableIndex": 58,
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));
    one_patches.push(make_event_state_info_patch(
        &class_indexes[7],
        flags,
        &class_indexes[8],
        &class_indexes[9],
        priority,
        &npc_event, // pa_$Ekm$
    ));

    one_patches.push({
        // "payload": "#$:AnimObj+&ao1$"
        let anim_obj_class_index = class_index_to_anim_object_map.get(&0).map(|v| v.clone());
        new_event_property_array(flags, anim_obj_class_index, &class_indexes[8], priority)
    });
    one_patches.push(new_synchronized_clip_generator(
        &class_indexes[9],
        npc_event.as_str(),
        &class_indexes[10],
        priority,
    ));

    one_patches.push((
        vec![
            Cow::Owned(class_indexes[10].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[10],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("Paired_FNISkm{priority}"), // FIXME?: priority <- $1/1$
                    "animationName": anim_file, // Animations\\$Fkm$
                    "triggers": &class_indexes[11],
                    "cropStartAmountLocalTime": 0.0,
                    "cropEndAmountLocalTime": 0.0,
                    "startTime": 0.0,
                    "playbackSpeed": 1.0,
                    "enforcedDuration": 0.0,
                    "userControlledTimeFraction": 0.0,
                    "animationBindingIndex": -1,
                    "mode": "MODE_SINGLE_PLAY",
                    "flags": 0
                }),
            },
            priority,
        },
    ));
    one_patches.push({
        // "id": 615, // NPCKillMoveStart
        let mut triggers =
            new_values_from_triggers(615, &paired_and_kill_animation.flag_set.triggers);

        //  156: NPCPairedStop
        //  616: NPCKillMoveEnd
        // 1070: NPCPairEnd (159: PairEnd)
        triggers.par_extend([156, 616, 1070].par_iter().map(|&id| {
            json_typed!(borrowed, {
                "localTime": duration, // $-D$
                "event": {
                    "id": id,
                    "payload": "#0000"
                },
                "relativeToEndOfClip": false,
                "acyclic": false,
                "isAnnotation": false
            })
        }));

        (
            vec![
                Cow::Owned(class_indexes[11].clone()),
                Cow::Borrowed("hkbClipTriggerArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[11],
                        "triggers": triggers
                    }),
                },
                priority,
            },
        )
    });
    one_patches.push(make_npc_root_state_info_patch(
        &class_indexes[12],
        &class_indexes[13],
        flags,
        priority,
        npc_root_state_name,
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[13].clone()),
            Cow::Borrowed("hkbModifierGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[13],
                    "variableBindingSet": "#0000",
                    "userData": 1,
                    "name": format!("NPC_FNISkm{priority}_ModGen"),
                    "modifier": &class_indexes[14],
                    "generator": &class_indexes[16]
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[14].clone()),
            Cow::Borrowed("BSIsActiveModifier"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[14],
                    "variableBindingSet": &class_indexes[15],
                    "userData": 2,
                    "name": format!("FNISkm{priority}_ActiveModifier"),
                    "enable": true,
                    "bIsActive0": false,
                    "bInvertActive0": false,
                    "bIsActive1": false,
                    "bInvertActive1": false,
                    "bIsActive2": false,
                    "bInvertActive2": false,
                    "bIsActive3": false,
                    "bInvertActive3": false,
                    "bIsActive4": false,
                    "bInvertActive4": false
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[15].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[15],
                    "bindings": [
                        {
                            "memberPath": "bIsActive0",
                            "variableIndex": 213,
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[16].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[16],
                    "variableBindingSet": &class_indexes[17],
                    "userData": 0,
                    "name": format!("NPC_FNISkm{priority}_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": {
                        "id": -1,
                        "payload": "#0000"
                    },
                    "startStateChooser": "#0000",
                    "startStateId": 0,
                    "returnToPreviousStateEventId": -1,
                    "randomTransitionEventId": -1,
                    "transitionToNextHigherStateEventId": -1,
                    "transitionToNextLowerStateEventId": -1,
                    "syncVariableIndex": -1,
                    "wrapAroundStateId": false,
                    "maxSimultaneousTransitions": 32,
                    "startStateMode": "START_STATE_MODE_DEFAULT",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                    "states": [&class_indexes[18]],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[17].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[17],
                    "bindings": [
                        {
                            "memberPath": "bIsActive0",
                            "variableIndex": 51,
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[18].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[18],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[19],
                    "name": format!("NPC_FNISkm{priority}_DisablePitch"),
                    "stateId": 0,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[19].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[19],
                    "variableBindingSet": &class_indexes[20],
                    "userData": 0,
                    "name": format!("NPC_FNISkm{priority}_DisablePitch_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": {
                        "id": -1,
                        "payload": "#0000"
                    },
                    "startStateChooser": "#0000",
                    "startStateId": 0,
                    "returnToPreviousStateEventId": -1,
                    "randomTransitionEventId": -1,
                    "transitionToNextHigherStateEventId": -1,
                    "transitionToNextLowerStateEventId": -1,
                    "syncVariableIndex": -1,
                    "wrapAroundStateId": false,
                    "maxSimultaneousTransitions": 32,
                    "startStateMode": "START_STATE_MODE_DEFAULT",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                    "states": [&class_indexes[21]],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[20].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[20],
                    "bindings": [
                        {
                            "memberPath": "isActive",
                            "variableIndex": 58,
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));
    one_patches.push(make_event_state_info_patch(
        &class_indexes[21],
        flags,
        &class_indexes[22],
        &class_indexes[23],
        priority,
        player_event,
    ));
    one_patches.push({
        // "payload": "#$:AnimObj+&ao2$"
        let anim_obj_class_index = class_index_to_anim_object_map.get(&1).map(|v| v.clone());
        new_event_property_array(flags, anim_obj_class_index, &class_indexes[22], priority)
    });
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[23].clone()),
            Cow::Borrowed("BSSynchronizedClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[23],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": player_event,
                    "pClipGenerator": &class_indexes[24],
                    "SyncAnimPrefix": "2_",
                    "bSyncClipIgnoreMarkPlacement": false,
                    "fGetToMarkTime": 0.0,
                    "fMarkErrorThreshold": 0.1,
                    "bLeadCharacter": true,
                    "bReorientSupportChar": true,
                    "bApplyMotionFromRoot": false,
                    "sAnimationBindingIndex": -1
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[24].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[24],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("NPCPaired_FNISkm{priority}"),
                    "animationName": anim_file, // Animations\\$Fkm$
                    "triggers": &class_indexes[25],
                    "cropStartAmountLocalTime": 0.0,
                    "cropEndAmountLocalTime": 0.0,
                    "startTime": 0.0,
                    "playbackSpeed": 1.0,
                    "enforcedDuration": 0.0,
                    "userControlledTimeFraction": 0.0,
                    "animationBindingIndex": -1,
                    "mode": "MODE_SINGLE_PLAY",
                    "flags": 0
                }),
            },
            priority,
        },
    ));
    one_patches.push({
        // Syntaxes starting with `T2`, such as `T2_KillActor/5.867`, are processed here.
        let mut triggers =
            new_values_from_triggers(614, &paired_and_kill_animation.flag_set.triggers2);

        // NOTE: The insertion of the following triggers is not documented even in 0_master.TEMPLATE.txt.
        //       This was discovered by reading the differences from `temporary_logs/0_master.xml`.
        // - event index list
        //    622: 2_KillMoveEnd
        //    167: 2_KillActor
        //   1120: 2_PairEnd
        triggers.par_extend([622, 167, 1120].par_iter().map(|&id| {
            json_typed!(borrowed, {
                "localTime": duration,
                "event": {
                    "id": id,
                    "payload": "#0000"
                },
                "relativeToEndOfClip": false,
                "acyclic": false,
                "isAnnotation": false
            })
        }));

        (
            vec![
                Cow::Owned(class_indexes[25].clone()),
                Cow::Borrowed("hkbClipTriggerArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[25],
                        "triggers": triggers,
                    }),
                },
                priority,
            },
        )
    });

    (one_patches, seq_patches)
}

/// - `state_name`:  e.g. `Player_FNIS_km{priority}`, `Player_FNISpa$1/1$`
///
/// # Note
/// - `enter_notify_events`: #2530 or null
/// - `exit_notify_events`: #2532 or null
#[must_use]
pub fn make_player_root_state_info_patch<'a>(
    class_index: &str,
    generator_index: &str,
    flags: FNISAnimFlags,
    priority: usize,
    state_name: String,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let enter_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraSet) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2530,
        false => "#0000",
    };
    let exit_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraReset) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2532,
        false => "#0000",
    };

    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_index,
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": enter_notify_events,
                    "exitNotifyEvents": exit_notify_events,
                    "transitions": "#0000",
                    "generator": generator_index,
                    "name": state_name,
                    "stateId": calculate_hash(&state_name),
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    )
}

/// - `state_name`:  e.g. `pa_{event_name}`, `pa_$Ekm` -> `pa_back`
///
/// # Note
/// - `enter_notify_events`: index or null or #2526
/// - `exit_notify_events`: `$-h,o|#2528|h|null|o|#2529|#2527$`
#[must_use]
pub fn make_event_state_info_patch<'a>(
    class_index: &str,
    flags: FNISAnimFlags,
    enter_notify_events_index: &str,
    generator_index: &str,
    priority: usize,
    state_name: &str,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects) {
        enter_notify_events_index
    } else if flags.contains(FNISAnimFlags::HeadTracking) {
        "#0000"
    } else {
        FNIS_AA_GLOBAL_AUTO_GEN_2526
    };
    // $-h,o|#2528|h|null|o|#2529|#2527$
    let exit_notify_events =
        if flags.contains(FNISAnimFlags::HeadTracking | FNISAnimFlags::AnimObjects) {
            FNIS_AA_GLOBAL_AUTO_GEN_2528
        } else if flags.contains(FNISAnimFlags::HeadTracking) {
            "#0000"
        } else if flags.contains(FNISAnimFlags::AnimObjects) {
            FNIS_AA_GLOBAL_AUTO_GEN_2529
        } else {
            FNIS_AA_GLOBAL_AUTO_GEN_2527
        };

    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_index,
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": enter_notify_events,
                    "exitNotifyEvents": exit_notify_events,
                    "transitions": "#0000",
                    "generator": generator_index,
                    "name": state_name,
                    "stateId": 0,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    )
}

/// - `state_name`:  e.g. `NPC_FNIS_km{priority}`, `NPC_FNISpa$1/1$`
#[must_use]
pub fn make_npc_root_state_info_patch<'a>(
    class_index: &str,
    generator_index: &str,
    flags: FNISAnimFlags,
    priority: usize,
    state_name: String,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    // $-ac1|#2534|#2533$
    let enter_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraSet) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2534,
        false => FNIS_AA_GLOBAL_AUTO_GEN_2533,
    };
    // $-ac0|#2532|null$
    let exit_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraReset) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2532,
        false => "#0000",
    };

    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_index,
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": enter_notify_events,
                    "exitNotifyEvents": exit_notify_events,
                    "transitions": "#0000",
                    "generator": generator_index,
                    "name": state_name,
                    "stateId": calculate_hash(&state_name),
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    )
}

#[must_use]
pub fn new_synchronized_clip_generator<'a>(
    class_index: &String,
    event: &str,
    generator_index: &str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    (
        vec![
            Cow::Owned(class_index.clone()),
            Cow::Borrowed("BSSynchronizedClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_index,
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": event,
                    "pClipGenerator": generator_index,
                    // See: https://github.com/SARDONYX-sard/serde-hkx/blob/main/crates/havok_types/src/string_ptr.rs#L181
                    "SyncAnimPrefix": null, // <- XML(&#9216;) null symbol to json
                    "bSyncClipIgnoreMarkPlacement": false,
                    "fGetToMarkTime": 0.0,
                    "fMarkErrorThreshold": 0.1,
                    "bLeadCharacter": false,
                    "bReorientSupportChar": true,
                    "bApplyMotionFromRoot": false,
                    "sAnimationBindingIndex": -1
                }),
            },
            priority,
        },
    )
}

#[must_use]
pub fn new_event_property_array<'a>(
    flags: FNISAnimFlags,
    anim_object_index: Option<String>,
    class_index: &str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = if flags.contains(FNISAnimFlags::HeadTracking) {
        vec![]
    } else {
        vec![
            json_typed!(borrowed, {
                "id": 366, // HeadTrackingOff
                "payload": "#0000"
            });
            3
        ]
    };

    if let Some(anim_object_index) = anim_object_index {
        // 937: AnimObjLoad
        // 936: AnimObjDraw
        events.extend([937, 936].iter().map(|id| {
            json_typed!(borrowed, {
                    "id": id,
                    "payload": anim_object_index
            })
        }));
    }

    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbStateMachineEventPropertyArray"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_index,
                    "events": events
                }),
            },
            priority,
        },
    )
}

#[must_use]
fn new_values_from_triggers<'a>(
    event_id: u64,
    triggers: &[Trigger<'a>],
) -> Vec<simd_json::borrowed::Value<'a>> {
    let mut values: Vec<simd_json::borrowed::Value> = vec![];
    values.push(json_typed!(borrowed, {
        "localTime": 0.0,
        "event": {
            "id": event_id,
            "payload": "#0000"
        },
        "relativeToEndOfClip": false,
        "acyclic": false,
        "isAnnotation": false
    }));

    values.par_extend(triggers.par_iter().map(|Trigger { event, time }| {
        json_typed!(borrowed, {
            "localTime": time, // $&TT1$
            "event": {
                "id": format!("$eventID[{event}]$"), // use Nemesis eventID variable. instead of $&TAE1$
                "payload": "#0000"
            },
            "relativeToEndOfClip": false,
            "acyclic": false,
            "isAnnotation": false
        })
    }));

    values
}

/// stateID(i32) generator?
#[must_use]
pub fn calculate_hash<T: std::hash::Hash + ?Sized>(t: &T) -> i32 {
    let mut hasher = std::hash::DefaultHasher::new();
    t.hash(&mut hasher);
    (std::hash::Hasher::finish(&hasher) & (i32::MAX as u64)) as i32
}

/// This is a PairedAndKillMove and specific to `character/behaviors/0_master.xml`,
///
/// - `index`: `hkbStateMachineTransitionInfoArray` index. 0_master: `#0789`, mt_behavior: `#4038`
/// - `events`: the actual event names
///   - e.g. `["back_grab", "pa_back_grab"]`
/// - `root_events`:
///   - e.g. `["Player_FNISkm{}", "NPC_FNISkm{}"]`
///
/// # Why is this needed?
/// The array `hkbStateMachineTransitionInfoArray.transitions` links
/// an **animation event (`eventId`)** to the **root state (`toStateId`)**
/// of the state machine.
///
/// This makes it possible to trigger a transition when a kill move event fires.
///
/// ## Mapping
/// | Role   | Example `eventId`       | Example `toStateId` (root state) |
/// |--------|-------------------------|----------------------------------|
/// | Player | `back_grab`             | `Player_FNISkm{}`                |
/// | NPC    | `pa_back_grab`          | `NPC_FNISkm{}`                   |
///
/// ## Flow
/// ```text
/// anim_event (kill move)
///        │
///        ▼
///   eventId (index of hkbSBehaviorStringData.eventNames)
///        │
///        ▼
///   toStateId (root hkbStateMachineInfo.stateId)
///        │
///        ▼
///   Transition executes → state machine moves to correct root state
/// ```
///
/// ## Crash Warning
/// If `eventId` and `toStateId` do not correctly match the root
/// `hkbStateMachineInfo`, the game will **crash instantly**
/// at the moment the animation is played.
pub fn new_push_transitions_seq_patch<'a>(
    index: &'static str,
    events: [&str; 2],
    root_state_names: [&String; 2],
    priority: usize,
) -> (json_path::JsonPath<'a>, ValueWithPriority<'a>) {
    let transitions: Vec<_> = events
        .iter()
        .zip(root_state_names.iter())
        .map(|(event_name, root_state_name)| {
            json_typed!(borrowed, {
                "triggerInterval": {
                    "enterEventId": -1,
                    "exitEventId": -1,
                    "enterTime": 0.0,
                    "exitTime": 0.0
                },
                "initiateInterval": {
                    "enterEventId": -1,
                    "exitEventId": -1,
                    "enterTime": 0.0,
                    "exitTime": 0.0
                },
                "transition": "#0111",
                "condition": "#0000",
                // eventId is Nemesis variable, derived from `events`
                "eventId": format!("$eventID[{event_name}]$"),
                // toStateId must match root_event (NOT the event)
                "toStateId": calculate_hash(root_state_name),
                "fromNestedStateId": 0,
                "toNestedStateId": 0,
                "priority": 0,
                "flags": "FLAG_IS_LOCAL_WILDCARD|FLAG_IS_GLOBAL_WILDCARD|FLAG_DISABLE_CONDITION"
            })
        })
        .collect();

    (
        json_path![index, "hkbStateMachineTransitionInfoArray", "transitions"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, transitions),
            },
            priority,
        },
    )
}
