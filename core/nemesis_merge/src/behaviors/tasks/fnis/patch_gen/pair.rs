use std::borrow::Cow;

use fnis_list::{
    combinator::{flags::FNISAnimFlags, Trigger},
    patterns::pair_and_kill::{ActorRole, AnimObject, FNISPairedAndKillAnimation},
};
use json_patch::{json_path, Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        global::_0_master::{FNIS_AA_GLOBAL_AUTO_GEN_2530, FNIS_AA_GLOBAL_AUTO_GEN_2532},
        kill_move::{
            calculate_hash, make_event_state_info_patch, make_player_root_state_info_patch,
            new_event_property_array, new_npc_synchronized_clip_generator,
            new_player_synchronized_clip_generator, new_push_transitions_seq_patch,
        },
        JsonPatchPairs,
    },
};

/// Into `meshes\actors\character\behaviors\0_master.xml`.
pub fn new_pair_patches<'a>(
    paired_and_kill_animation: FNISPairedAndKillAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [String; 22] =
        std::array::from_fn(|_| owned_data.next_class_name_attribute());

    let class_index_0_id = calculate_hash(&class_indexes[0]); // Must be 1 file unique
    let class_index_12_id = calculate_hash(&class_indexes[12]); // Must be 1 file unique

    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let flags = paired_and_kill_animation.flag_set.flags;

    let player_event = paired_and_kill_animation.anim_event;
    let npc_event = format!("pa_{class_index_0_id}");
    let duration = paired_and_kill_animation.flag_set.duration;
    let anim_file = format!(
        "Animations\\{namespace}\\{}",
        paired_and_kill_animation.anim_file
    ); // Animations\\$Fkm$

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    let player_root_state_name = format!("Player_FNISpa{class_index_0_id}"); // NOTE: must be unique in 0_master.xml
    let npc_root_state_name = format!("NPC_FNISpa{class_index_12_id}"); // NOTE: must be unique in 0_master.xml

    seq_patches.push(new_push_transitions_seq_patch(
        "#0789",
        "#0111",
        [player_event, npc_event.as_str()],
        [&player_root_state_name, &npc_root_state_name],
        priority,
    ));

    // Push and register the Root `hkbStateMachineStateInfo` for both Player and NPC.
    seq_patches.push((
        json_path!["#0788", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::SeqPush,
                value: json_typed!(borrowed, [class_indexes[0], class_indexes[12]]),
            },
            priority,
        },
    ));

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let (active_indexes, passive_indexes, patches) = paired_and_kill_animation
        .anim_objects
        .par_iter()
        .fold(
            || (Vec::new(), Vec::new(), Vec::new()),
            |mut acc, AnimObject { name, role }| {
                let new_anim_object_index = owned_data.next_class_name_attribute();

                acc.2.push((
                    vec![
                        Cow::Owned(new_anim_object_index.clone()),
                        Cow::Borrowed("hkbStringEventPayload"),
                    ],
                    ValueWithPriority {
                        patch: JsonPatch {
                            action: Action::Pure { op: Op::Add },
                            value: simd_json::json_typed!(borrowed, {
                                "__ptr": new_anim_object_index,
                                "data": name, // StringPtr
                            }),
                        },
                        priority,
                    },
                ));

                match role {
                    ActorRole::Active => acc.0.push(new_anim_object_index),
                    ActorRole::Passive => acc.1.push(new_anim_object_index),
                }

                acc
            },
        )
        .reduce(
            || (Vec::new(), Vec::new(), Vec::new()),
            |mut a, b| {
                a.0.extend(b.0);
                a.1.extend(b.1);
                a.2.extend(b.2);
                a
            },
        );
    one_patches.par_extend(patches);

    // $RI
    one_patches.push({
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
                Cow::Owned(class_indexes[0].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[0],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": "#0000",
                        "generator": &class_indexes[1],
                        "name": player_root_state_name, // Player_FNISpa$1/1$
                        "stateId": calculate_hash(&player_root_state_name), // $171/2$
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
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[1],
                    "variableBindingSet": class_indexes[2],
                    "userData": 0,
                    "name": format!("Player_FNISpa{class_index_0_id}_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": [{
                        "id": -1,
                        "payload": "#0000"
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
                    "states": [class_indexes[3]],
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
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[2],
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

    // #$RI+3$  hkbStateMachineStateInfo
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[3].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[3],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[4],
                    "name": format!("Player_FNISpa{class_index_0_id}_DisablePitch"),
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
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[4],
                    "variableBindingSet": &class_indexes[5],
                    "userData": 0,
                    "name": format!("Player_FNISpa{class_index_0_id}_DisablePitch_Behavior"),
                    "eventToSendWhenStateOrTransitionChanges": [{
                        "id": -1,
                        "payload": "#0000",
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
                    "states": [&class_indexes[6]],
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
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[5],
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

    // #$RI+6$  hkbStateMachineStateInfo
    one_patches.push(make_event_state_info_patch(
        &class_indexes[6],
        flags,
        &class_indexes[7],
        &class_indexes[8],
        priority,
        &npc_event, // FNISpa_$1/1$
    ));

    // #$RI+7$  hkbStateMachineEventPropertyArray
    one_patches.push({
        // "payload": "#$:AnimObj+&ao1$"
        new_event_property_array(flags, &active_indexes, &class_indexes[7], priority)
    });

    // #$RI+8$  BSSynchronizedClipGenerator
    one_patches.push(new_npc_synchronized_clip_generator(
        &class_indexes[8],
        npc_event.as_str(),
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
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[9],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("Paired_FNISpa{class_index_0_id}"),
                    "animationName": anim_file,
                    "triggers": &class_indexes[10],
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
        //  159: PairEnd
        // 1070: NPCPairEnd
        triggers.par_extend([156, 1070].par_iter().map(|&id| {
            json_typed!(borrowed, {
                "localTime": duration, // $-D$
                "event": {
                    "id": id, // The details are unclear, but 159 was not used—only 1070 was being used.
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
                    action: Action::Pure { op: Op::Add },
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[10],
                        "triggers": triggers
                    }),
                },
                priority,
            },
        )
    });

    one_patches.push(make_player_root_state_info_patch(
        &class_indexes[11],
        &class_indexes[12],
        flags,
        priority,
        npc_root_state_name,
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[12].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[12],
                    "variableBindingSet": class_indexes[13],
                    "userData": 0,
                    "name": format!("NPC_FNISpa{class_index_0_id}$_Behavior"),
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
                action: Action::Pure { op: Op::Add },
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
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[14],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[15],
                    "name": format!("NPC_FNISpa{class_index_0_id}_DisablePitch"),
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
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[15],
                    "variableBindingSet": &class_indexes[16],
                    "userData": 0,
                    "name": format!("NPC_FNISpa{class_index_0_id}_DisablePitch_Behavior"),
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
                    "states": [&class_indexes[17]],
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
                action: Action::Pure { op: Op::Add },
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
    one_patches.push(make_event_state_info_patch(
        &class_indexes[17],
        flags,
        &class_indexes[18],
        &class_indexes[19],
        priority,
        &npc_event, // FNISpa_$1/1$
    ));
    one_patches.push({
        // "payload": "#$:AnimObj+&ao2$"
        new_event_property_array(flags, &passive_indexes, &class_indexes[18], priority)
    });
    // $RI+19
    one_patches.push(new_player_synchronized_clip_generator(
        &class_indexes[19],
        player_event,
        &class_indexes[20],
        priority,
    ));
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[20].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[20],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("NPC_Paired_FNISpa{class_index_0_id}"),
                    "animationName": anim_file,
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
                    action: Action::Pure { op: Op::Add },
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
