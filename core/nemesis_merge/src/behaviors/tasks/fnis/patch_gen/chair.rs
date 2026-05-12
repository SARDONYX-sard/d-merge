//! To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
//!
//! # How to Read the FNIS Template for Chair
use std::borrow::Cow;

use fnis_list::patterns::chair::FNISChairAnimation;
use json_patch::{Action, JsonPatch, Op, ValueWithPriority, json_path};
use rayon::prelude::*;
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        JsonPatchPairs, furniture::one_anim::new_push_values_seq_patch, kill_move::calculate_hash,
        new_push_events_seq_patch,
    },
};

/// Creates JSON patches for a single chair animation, depending on its phase.
///
/// - `has_any_anim_obj`: Whether any furniture sequence contains an animation object
///
/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub(super) fn new_chair_patches<'a>(
    chair: &FNISChairAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [String; 19] =
        core::array::from_fn(|_| owned_data.next_class_name_attribute());

    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;

    // NOTE: Must be 1 file unique ID
    let class_index_0_id = calculate_hash(&class_indexes[0]);
    let anim_files = build_anim_files(namespace, chair);

    let start_animation = &chair.start;
    let start_event = start_animation.anim_event;

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.extend(new_push_events_seq_patch(
        &[start_event.into()],
        "#0083",
        "#0085",
        priority,
    ));
    if !start_animation.flag_set.anim_vars.is_empty() {
        seq_patches.par_extend(new_push_values_seq_patch(
            &start_animation.flag_set.anim_vars,
            "#0083",
            "#0084",
            "#0085",
            priority,
        ));
    }

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let class_index_to_anim_object_map = dashmap::DashMap::new();
    one_patches.par_extend(start_animation.anim_objects.par_iter().enumerate().map(
        |(index, name)| {
            let new_anim_object_index = owned_data.next_class_name_attribute();
            class_index_to_anim_object_map.insert(index, new_anim_object_index.clone());

            // One anim object
            (
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
            )
        },
    ));

    // $RI
    one_patches.push((
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
                    "enterNotifyEvents": &class_indexes[1],
                    "exitNotifyEvents": "#1731",
                    "transitions": "#1730",
                    "generator": &class_indexes[2],
                    "name": format!("FNISChairIdleState{class_index_0_id}"),
                    "stateId": class_index_0_id, // $101/1$
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+1
    one_patches.push(new_event_property_array_ri1(
        &class_indexes[1],
        &class_index_to_anim_object_map,
        priority,
    ));

    // $RI+2
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[2].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[2],
                    "variableBindingSet": class_indexes[3],
                    "userData": 0,
                    "name": format!("FNISChairIdleBehavior{class_index_0_id}"), // FNISChairIdleBehavior$1/1$
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
                    "states": [class_indexes[4], class_indexes[7], class_indexes[15]],
                    "wildcardTransitions": "#1720"
                }),
            },
            priority,
        },
    ));

    // $RI+3
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[3].clone()),
            Cow::Borrowed("hkbVariableBindingSet"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                        "__ptr": class_indexes[3],
                        "bindings": [
                            {
                                "memberPath": "isActive",
                                "variableIndex": 60, // bTalkableWithItem
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

    // $RI+4
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[4].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[4],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[5],
                    "name": format!("FNISChairIdleStart{class_index_0_id}"), // FNISChairIdleStart$1/1$
                    "stateId": 0,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+5
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[5].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[5],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("FNISChairIdleStart{class_index_0_id}"), // FNISChairIdleStart$1/1$
                    "animationName": anim_files[0], // Animations\\$chg-4$
                    "triggers": &class_indexes[6],
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

    // $RI+6
    one_patches.push({
        let mut triggers = Vec::with_capacity(1 + class_index_to_anim_object_map.len());
        triggers.push(json_typed!(borrowed, {
            "localTime": -0.2,
            "event": {
                "id": 91, // ChairNextClip
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));

        triggers.extend(class_index_to_anim_object_map.iter().map(|ref_| {
            json_typed!(borrowed, {
                "localTime": -0.2,
                "event": {
                    "id": 394, // AnimObjDraw
                    "payload": ref_.value(),
                },
                "relativeToEndOfClip": true,
                "acyclic": false,
                "isAnnotation": false
            })
        }));

        (
            vec![
                Cow::Owned(class_indexes[6].clone()),
                Cow::Borrowed("hkbClipTriggerArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[6],
                        "triggers": triggers
                    }),
                },
                priority,
            },
        )
    });

    // $RI+7
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[7].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[7],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[8],
                    "name": format!("FNISChairIdle{class_index_0_id}"), // FNISChairIdle$1/1$
                    "stateId": 1,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+8
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[8].clone()),
            Cow::Borrowed("hkbStateMachine"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[8],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("FNISChairIdleBehavior{class_index_0_id}"), // FNISChairIdleBehavior$1/1$
                    "eventToSendWhenStateOrTransitionChanges": {
                        "id": -1,
                        "payload": "#0000"
                    },
                    "startStateChooser": "#0000",
                    "startStateId": 0,
                    "returnToPreviousStateEventId": -1,
                    "randomTransitionEventId": 91, // ChairNextClip
                    "transitionToNextHigherStateEventId": -1,
                    "transitionToNextLowerStateEventId": -1,
                    "syncVariableIndex": -1,
                    "wrapAroundStateId": false,
                    "maxSimultaneousTransitions": 32,
                    "startStateMode": "START_STATE_MODE_DEFAULT",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                    "states": [class_indexes[9], class_indexes[11], class_indexes[13]],
                    "wildcardTransitions": "#1117"
                }),
            },
            priority,
        },
    ));

    // $RI+9
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[9].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[9],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#0000",
                    "generator": &class_indexes[10],
                    "name": format!("FNISChairIdleBase{class_index_0_id}"), // FNISChairIdleBase$1/1$
                    "stateId": 0,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+10
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[10].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[10],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("FNISChairIdleBase{class_index_0_id}"), // FNISChairIdleBase$1/1$
                    "animationName": anim_files[1], // Animations\\$chg-3$
                    "triggers": &class_indexes[6],
                    "cropStartAmountLocalTime": 0.0,
                    "cropEndAmountLocalTime": 0.0,
                    "startTime": 0.0,
                    "playbackSpeed": 1.0,
                    "enforcedDuration": 0.0,
                    "userControlledTimeFraction": 0.0,
                    "animationBindingIndex": -1,
                    "mode": "MODE_LOOPING",
                    "flags": 0
                }),
            },
            priority,
        },
    ));

    // $RI+11
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[11].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[11],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#1127",
                    "exitNotifyEvents": "#1126",
                    "transitions": "#0000",
                    "generator": &class_indexes[12],
                    "name": format!("FNISChairIdleVar1_{class_index_0_id}"), // FNISChairIdleVar1_$1/1$
                    "stateId": 1,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+12
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[12].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[12],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("FNISChairIdleVar1_{class_index_0_id}"), // FNISChairIdleVar1_$1/1$
                    "animationName": anim_files[2], // Animations\\$chg-2$
                    "triggers": "#5222",
                    "cropStartAmountLocalTime": 0.0,
                    "cropEndAmountLocalTime": 0.0,
                    "startTime": 0.0,
                    "playbackSpeed": 1.0,
                    "enforcedDuration": 0.0,
                    "userControlledTimeFraction": 0.0,
                    "animationBindingIndex": -1,
                    "mode": "MODE_LOOPING",
                    "flags": 0
                }),
            },
            priority,
        },
    ));

    // $RI+13
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[13].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[13],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#1127",
                    "exitNotifyEvents": "#1126",
                    "transitions": "#0000",
                    "generator": &class_indexes[12],
                    "name": format!("FNISChairIdleVar2_{class_index_0_id}"), // FNISChairIdleVar2_$1/1$
                    "stateId": 2,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+14
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[14].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[14],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("FNISChairIdleVar2_{class_index_0_id}"), // FNISChairIdleVar2_$1/1$
                    "animationName": anim_files[3], // Animations\\$chg-1$
                    "triggers": "#5222",
                    "cropStartAmountLocalTime": 0.0,
                    "cropEndAmountLocalTime": 0.0,
                    "startTime": 0.0,
                    "playbackSpeed": 1.0,
                    "enforcedDuration": 0.0,
                    "userControlledTimeFraction": 0.0,
                    "animationBindingIndex": -1,
                    "mode": "MODE_LOOPING",
                    "flags": 0
                }),
            },
            priority,
        },
    ));

    // $RI+15
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[15].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[15],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#1721",
                    "generator": &class_indexes[16],
                    "name": format!("FNISChairTalkingState{class_index_0_id}"), // FNISChairTalkingState$1/1$
                    "stateId": 2,
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // $RI+16
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[16].clone()),
            Cow::Borrowed("hkbModifierGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[16],
                    "variableBindingSet": "#0000", // null ptr
                    "userData": 1,
                    "name": format!("FNISChairIdle_MG{class_index_0_id}"), // FNISChairIdle_MG$1/1$
                    "modifier": class_indexes[17],
                    "generator": class_indexes[18],
                }),
            },
            priority,
        },
    ));

    // $RI+17
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[17].clone()),
            Cow::Borrowed("BSEventEveryNEventsModifier"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[17],
                    "variableBindingSet": "#0000",
                    "userData": 3,
                    "name": format!("BSEventEveryNEventsMod_5and2_{class_index_0_id}"), // BSEventEveryNEventsMod_5and2_$1/1$
                    "enable": true,
                    "eventToCheckFor": {
                        "id": 168, // CountDownTick
                        "payload": "#0000"
                    },
                    "eventToSend": {
                        "id": 132, // 00NextClip
                        "payload": "#0000"
                    },
                    "numberOfEventsBeforeSend": 5,
                    "minimumNumberOfEventsBeforeSend": 2,
                    "randomizeNumberOfEvents": true,
                }),
            },
            priority,
        },
    ));

    // $RI+18
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[18].clone()),
            Cow::Borrowed("hkbClipGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_indexes[18],
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": format!("FNISChairTalking{class_index_0_id}"), // FNISChairTalking$1/1$
                    "animationName": anim_files[4], // Animations\\$chg-3$
                    "triggers": "#1114",
                    "cropStartAmountLocalTime": 0.0,
                    "cropEndAmountLocalTime": 0.0,
                    "startTime": 0.0,
                    "playbackSpeed": 1.0,
                    "enforcedDuration": 0.0,
                    "userControlledTimeFraction": 0.0,
                    "animationBindingIndex": -1,
                    "mode": "MODE_LOOPING",
                    "flags": 0
                }),
            },
            priority,
        },
    ));

    // Push the first animation for chair seq
    seq_patches.push({
        (
            json_patch::json_path!["#1227", "hkbStateMachineTransitionInfoArray", "transitions"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: json_typed!(borrowed, [{
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
                        "transition": "#0091",
                        "condition": "#0000",
                        "eventId": format!("$eventID[{start_event}]$"), // eventId is Nemesis variable
                        "toStateId": class_index_0_id, // Must match root_state
                        "fromNestedStateId": 0,
                        "toNestedStateId": 0,
                        "priority": priority,
                        "flags": "FLAG_DISABLE_CONDITION"
                    }]),
                },
                priority,
            },
        )
    });

    seq_patches.push((
        json_path!["#1755", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::SeqPush,
                value: json_typed!(borrowed, [class_indexes[0]]),
            },
            priority,
        },
    ));

    (one_patches, seq_patches)
}

#[must_use]
fn new_event_property_array_ri1<'a>(
    class_index: &str,
    class_index_to_anim_object_map: &dashmap::DashMap<usize, String>,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let events: Vec<_> = class_index_to_anim_object_map
        .iter()
        .map(|ref_| {
            json_typed!(borrowed, {
                "id": 393, // AnimObjLoad
                "payload": ref_.value() // #$:AnimObj+&ao$
            })
        })
        .collect();

    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbStateMachineEventPropertyArray"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_index,
                    "events": events
                }),
            },
            priority,
        },
    )
}

/// Builds exactly 5 animation file paths.
///
/// Missing sequenced animations reuse the last available animation.
fn build_anim_files(namespace: &str, chair: &FNISChairAnimation) -> [String; 5] {
    let FNISChairAnimation { start, sequenced } = chair;
    let start_anim_file = start.anim_file;

    [
        format!("Animations\\{namespace}\\{start_anim_file}"),
        format!(
            "Animations\\{namespace}\\{}",
            sequenced.first().unwrap_or(&start_anim_file)
        ),
        format!(
            "Animations\\{namespace}\\{}",
            sequenced
                .get(1)
                .or_else(|| sequenced.first())
                .unwrap_or(&start_anim_file)
        ),
        format!(
            "Animations\\{namespace}\\{}",
            sequenced
                .get(2)
                .or_else(|| sequenced.get(1))
                .or_else(|| sequenced.first())
                .unwrap_or(&start_anim_file)
        ),
        format!(
            "Animations\\{namespace}\\{}",
            sequenced
                .get(3)
                .or_else(|| sequenced.get(2))
                .or_else(|| sequenced.get(1))
                .or_else(|| sequenced.first())
                .unwrap_or(&start_anim_file)
        ),
    ]
}
