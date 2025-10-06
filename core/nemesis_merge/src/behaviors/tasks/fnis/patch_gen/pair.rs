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
    kill_move::{
        calculate_hash, get_anim_object_index, make_state_info_patch, make_state_info_patch2,
        new_event_property_array, new_synchronized_clip_generator,
    },
    JsonPatchPairs, PUSH_OP,
};

/// Into `meshes\actors\character\behaviors\0_master.xml`.
pub fn new_pair_patches<'a>(
    paired_and_kill_animation: FNISPairedAndKillAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [String; 23] =
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
                class_index_to_anim_object_map.insert(index, name);

                let new_anim_object_index = owned_data.next_class_name_attribute();
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

    // $RI
    one_patches.push({
        let enter_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraSet) {
            true => "#2530",
            false => "#0000",
        };
        let exit_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraReset) {
            true => "#2532",
            false => "#0000",
        };

        (
            vec![
                Cow::Owned(class_indexes[0].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[0],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": "#0000",
                        "generator": &class_indexes[1],
                        "name": format!("Player_FNISpa{priority}"),
                        "stateId": calculate_hash(&class_indexes[0]),
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });

    // RI+1
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[1].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[1],
                    "variableBindingSet": class_indexes[2],
                    "userData": 0,
                    "name": format!("Player_FNISpa{priority}_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": [{
                        "id": -1,
                        "payload": null
                    }],
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
                    "states": [ class_indexes[3] ],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));

    // #$RI+2$  hkbVariableBindingSet
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[2].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[2],
                    "bindings": [{
                        "memberPath": "isActive",
                        "variableIndex": 51,
                        "bitIndex": -1,
                        "bindingType": "BINDING_TYPE_VARIABLE"
                    }],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));

    // #$RI+3$  hkbStateMachineStateInfo
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[3].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[3],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[4],
                    "name": format!("Player_FNISpa{priority}_DisablePitch"),
                    "stateId": 0,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // #$RI+4$  hkbStateMachine
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[4].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[4],
                    "variableBindingSet": class_indexes[5],
                    "userData": 0,
                    "name": format!("Player_FNISpa{priority}_DisablePitch_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": [{
                        "id": -1,
                        "payload": null
                    }],
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
                    "states": [ class_indexes[6] ],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));
    // #$RI+5$  hkbVariableBindingSet
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[5].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[5],
                    "bindings": [{
                        "memberPath": "isActive",
                        "variableIndex": 58,
                        "bitIndex": -1,
                        "bindingType": "BINDING_TYPE_VARIABLE"
                    }],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));

    // #$RI+6$  hkbStateMachineStateInfo
    one_patches.push({
        // $-o|#%RI+7%|h|null|#2526$
        let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects) {
            class_indexes[7].as_str()
        } else if flags.contains(FNISAnimFlags::HeadTracking) {
            "#0000"
        } else {
            "#2526"
        };
        // $-h,o|#2528|h|null|o|#2529|#2527$
        let exit_notify_events =
            if flags.contains(FNISAnimFlags::HeadTracking | FNISAnimFlags::AnimObjects) {
                "#2528"
            } else if flags.contains(FNISAnimFlags::HeadTracking) {
                "#0000"
            } else if flags.contains(FNISAnimFlags::AnimObjects) {
                "#2529"
            } else {
                "#2527"
            };

        (
            vec![
                Cow::Owned(class_indexes[6].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[6],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": "#0000",
                        "generator": &class_indexes[8],
                        "name": format!("pa_FNISpa{priority}"),
                        "stateId": 0,
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });

    // #$RI+7$  hkbStateMachineEventPropertyArray
    one_patches.push({
        let first_anim_object_index = class_index_to_anim_object_map
            .get(&0)
            .map_or("#0000", |p| **p.value());
        new_event_property_array(first_anim_object_index, &class_indexes[7], priority)
    });

    // #$RI+8$  BSSynchronizedClipGenerator
    one_patches.push(new_synchronized_clip_generator(
        &class_indexes[8],
        namespace,
        &class_indexes[9],
        priority,
    ));

    // #$RI+9$  hkbClipGenerator
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[9].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[9],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("Paired_FNISpa{priority}"),
                    "animationName": anim_file,
                    "triggers": class_indexes[10],
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
        let mut triggers: Vec<_> = paired_and_kill_animation
            .flag_set
            .triggers
            .par_iter()
            .map(|Trigger { event, time }| {
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
            })
            .collect();

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
                Cow::Owned(class_indexes[10].clone()),
                Cow::Borrowed("hkbClipTriggerArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[10],
                        "triggers": triggers
                    }),
                },
                priority,
            },
        )
    });

    one_patches.push(make_state_info_patch(
        &class_indexes[11],
        &class_indexes[12],
        flags,
        priority,
        format!("NPC_FNISpa{priority}"),
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[12].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[12],
                    "variableBindingSet": class_indexes[13],
                    "userData": 0,
                    "name": format!("NPC_FNISpa{priority}$_Behavior"),
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
                    "states": [class_indexes[14]],
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[13].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[13],
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
            Cow::Owned(class_indexes[14].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[14],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[15],
                    "name": format!("NPC_FNISpa{priority}_DisablePitch"),
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
            Cow::Owned(class_indexes[15].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[15],
                    "variableBindingSet": &class_indexes[16],
                    "userData": 0,
                    "name": format!("NPC_FNISpa{priority}_DisablePitch_Behavior"),
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
            Cow::Owned(class_indexes[16].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[16],
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
        &class_indexes[17],
        flags,
        &class_indexes[18],
        &class_indexes[19],
        priority,
        format!("FNISpa_{namespace}"), // FNISpa_$1/1$
    ));
    one_patches.push({
        // "payload": "#$:AnimObj+&ao2$" (fallback to first)
        let maybe_2nd_anim_object_index = get_anim_object_index(&class_index_to_anim_object_map, 1);
        new_event_property_array(maybe_2nd_anim_object_index, &class_indexes[18], priority)
    });
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[19].clone()),
            Cow::Borrowed("BSSynchronizedClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[19],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": paired_and_kill_animation.anim_event,
                    "pClipGenerator": &class_indexes[20],
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
            Cow::Owned(class_indexes[20].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[20],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("NPC_Paired_FNISpa{priority}"), // FIXME?: priority <- $1/1$
                    "animationName": anim_file, // Animations\\$Fkm$
                    "triggers": &class_indexes[21],
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
        let triggers: Vec<_> = paired_and_kill_animation
            .flag_set
            .triggers
            .par_iter()
            .map(|Trigger { event, time }| {
                json_typed!(borrowed, {
                    "localTime": time, // $&TT2$
                    "event": {
                        "id": format!("$eventID[{event}]$"), // use Nemesis eventID variable. instead of $&TAE2$
                        "payload": "#0000"
                    },
                    "relativeToEndOfClip": false,
                    "acyclic": false,
                    "isAnnotation": false
                })
            })
            .collect();

        (
            vec![
                Cow::Owned(class_indexes[21].clone()),
                Cow::Borrowed("hkbClipTriggerArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[21],
                        "triggers": triggers
                    }),
                },
                priority,
            },
        )
    });

    (one_patches, seq_patches)
}
