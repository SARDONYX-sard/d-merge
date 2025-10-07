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
use crate::behaviors::tasks::fnis::patch_gen::{
    JsonPatchPairs, FNIS_AA_GLOBAL_AUTO_GEN_2526, FNIS_AA_GLOBAL_AUTO_GEN_2527,
    FNIS_AA_GLOBAL_AUTO_GEN_2528, FNIS_AA_GLOBAL_AUTO_GEN_2529, FNIS_AA_GLOBAL_AUTO_GEN_2530,
    FNIS_AA_GLOBAL_AUTO_GEN_2532, FNIS_AA_GLOBAL_AUTO_GEN_2533, FNIS_AA_GLOBAL_AUTO_GEN_2534,
    PUSH_OP,
};

/// Into `meshes\actors\character\behaviors\0_master.xml`.
pub fn new_kill_patches<'a>(
    paired_and_kill_animation: FNISPairedAndKillAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [String; 26] =
        std::array::from_fn(|_| owned_data.next_class_name_attribute());
    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let flags = paired_and_kill_animation.flag_set.flags;
    let duration = paired_and_kill_animation.flag_set.duration;
    let anim_file = format!(
        "Animations\\{namespace}\\{}",
        paired_and_kill_animation.anim_file
    ); // Animations\\$Fkm$

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.push((
        json_path!["#0788", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, class_indexes),
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

    one_patches.push(make_state_info_patch(
        &class_indexes[0],
        &class_indexes[1],
        flags,
        priority,
        format!("Player_FNISkm{priority}"),
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
                    "name": format!("Player_FNISkm{priority}_Behavior"), // StringPtr
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
    one_patches.push(make_state_info_patch2(
        &class_indexes[7],
        flags,
        &class_indexes[8],
        &class_indexes[9],
        priority,
        format!("pa_{}", paired_and_kill_animation.anim_event), // pa_$Ekm$
    ));

    one_patches.push({
        let first_anim_object_index = class_index_to_anim_object_map
            .get(&0)
            .map_or(Cow::Borrowed("#0000"), |p| Cow::Owned(p.value().clone()));
        new_event_property_array(&first_anim_object_index, &class_indexes[8], priority)
    });
    one_patches.push(new_synchronized_clip_generator(
        &class_indexes[9],
        namespace,
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
    one_patches.push({
        let enter_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraSet) {
            true => FNIS_AA_GLOBAL_AUTO_GEN_2534,
            false => FNIS_AA_GLOBAL_AUTO_GEN_2533,
        };
        let exit_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraReset) {
            true => FNIS_AA_GLOBAL_AUTO_GEN_2532,
            false => "#0000",
        };

        let state_name = format!("NPC_FNISkm{priority}");

        (
            vec![
                Cow::Owned(class_indexes[12].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[12],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": "#0000",
                        "generator": &class_indexes[13],
                        "name": state_name,
                        "stateId": calculate_hash(&state_name),
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });
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
                    "name": format!("FNISkm{priority}$_ActiveModifier"),
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
    one_patches.push({
        let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects) {
            class_indexes[22].as_str()
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
                Cow::Owned(class_indexes[21].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[21],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": "#0000",
                        "generator": &class_indexes[23],
                        "name": paired_and_kill_animation.anim_event,
                        "stateId": 0,
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });
    one_patches.push({
        // "payload": "#$:AnimObj+&ao2$" (fallback to first)
        let maybe_2nd_anim_object_index = get_anim_object_index(&class_index_to_anim_object_map, 1);
        new_event_property_array(&maybe_2nd_anim_object_index, &class_indexes[22], priority)
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
                    "name": paired_and_kill_animation.anim_event,
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
        let triggers = new_values_from_triggers(614, &paired_and_kill_animation.flag_set.triggers);

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

/// Retrieves the first available animation object index from the map,
/// starting from `start_index` and counting down to 0.
///
/// # Returns
///
/// A `Cow<str>` representing the first found value, or `"#0000"` if none exist.
#[must_use]
pub fn get_anim_object_index(
    map: &dashmap::DashMap<usize, String>,
    start_index: usize,
) -> Cow<'static, str> {
    for idx in (0..=start_index).rev() {
        if let Some(val) = map.get(&idx) {
            return Cow::Owned(val.value().clone());
        }
    }
    Cow::Borrowed("#0000")
}

/// - `state_name`:  e.g. `FNIS_State{priority}`, `Player_FNISpa$1/1$`
///
/// # Note
/// - `enter_notify_events`: #2530 or null
/// - `exit_notify_events`: #2532 or null
#[must_use]
pub fn make_state_info_patch<'a>(
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

/// - `state_name`:  e.g. `FNIS_State{priority}`, `Player_FNISpa$1/1$`
///
/// # Note
/// - `enter_notify_events`: index or null or #2526
/// - `exit_notify_events`: `$-h,o|#2528|h|null|o|#2529|#2527$`
#[must_use]
pub fn make_state_info_patch2<'a>(
    class_index: &str,
    flags: FNISAnimFlags,
    enter_notify_events_index: &str,
    generator_index: &str,
    priority: usize,
    state_name: String,
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
                    "stateId": calculate_hash(class_index),
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
    namespace: &'a str,
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
                    "name": format!("pa_{namespace}"),
                    "pClipGenerator": generator_index,
                    "SyncAnimPrefix": "\u{2400}", // <- XML(&#9216;) to unicode
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
    anim_object_index: &str,
    class_index: &str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
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
                    "events": [
                        {
                            "id": 366, // HeadTrackingOff
                            "payload": "#0000"
                        },
                        {
                            "id": 937, // AnimObjLoad
                            "payload": anim_object_index
                        },
                        {
                            "id": 936, // AnimObjDraw
                            "payload": anim_object_index
                        }
                    ]
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

/// stateID generator?
#[must_use]
pub fn calculate_hash<T: std::hash::Hash + ?Sized>(t: &T) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    t.hash(&mut hasher);
    std::hash::Hasher::finish(&hasher)
}
