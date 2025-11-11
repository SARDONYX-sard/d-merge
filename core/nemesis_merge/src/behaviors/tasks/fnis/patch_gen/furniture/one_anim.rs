//! To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
//!
//! # How to Read the FNIS Template for Furniture
//! - Note: This is my speculation and not definitive.
//!
//! As background, FNIS_modders.pdf states Furniture requires at least four animations. Meaning it's a special sequence.
//! `-F`: Start of the Furniture syntax sequence. The statement where `fu` is used.
//! `-L`: End of the Furniture syntax sequence (Last).
use std::borrow::Cow;

use json_patch::{json_path, Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;
use simd_json::{borrowed::Value, json_typed};

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::{AnimVar, FNISAnimFlags};
use crate::behaviors::tasks::fnis::list_parser::combinator::fnis_animation::FNISAnimation;
use crate::behaviors::tasks::fnis::list_parser::combinator::Trigger;
use crate::behaviors::tasks::fnis::patch_gen::global::mt_behavior::FNIS_BA_BLEND_TRANSITION_5231;
use crate::behaviors::tasks::fnis::patch_gen::new_push_events_seq_patch;
use crate::behaviors::tasks::fnis::patch_gen::{kill_move::calculate_hash, JsonPatchPairs};

/// Represents the phase of a Furniture animation sequence as defined in FNIS.
///
/// According to *FNIS_modders.pdf*, Furniture animations require at least four
/// animations forming a special sequence:
///
/// FNIS TEMPLATE.txt block
/// - `-F`(First): Start of the Furniture syntax sequence
/// - (middle entries): Continuation of the Furniture sequence
/// - `-L`(Last): End of the Furniture syntax sequence
///
/// This enum is used to identify which phase of the sequence is currently being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurniturePhase {
    /// The first phase of the Furniture animation sequence (`-F`).
    Start,

    /// A middle phase of the Furniture sequence.
    /// Contains an index (e.g., `Middle(1)`, `Middle(2)`).
    Middle(usize),

    /// The final phase of the Furniture animation sequence (`-L`).
    /// Contains an index
    End(usize),
}

/// Creates JSON patches for a single Furniture animation, depending on its phase.
///
/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub fn new_furniture_one_anim_patches<'a>(
    animation: &FNISAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
    current_phase: FurniturePhase,
    class_indexes: &[String; 9],
    end_anim_state_id: i32,
    next_event_name: Option<&'a str>,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let flags = animation.flag_set.flags;
    let event_name = animation.anim_event;

    let is_empty_anim_vars = animation.flag_set.anim_vars.is_empty();

    // NOTE: Must be 1 file unique ID
    let class_index_0_id = calculate_hash(&class_indexes[0]);

    let anim_file = format!("Animations\\{namespace}\\{}", animation.anim_file); // Animations\\$Foa$

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.extend(new_push_events_seq_patch(
        &[animation.anim_event.into()],
        "#0083",
        "#0085",
        priority,
    ));
    if !is_empty_anim_vars {
        seq_patches.par_extend(new_push_values_seq_patch(
            &animation.flag_set.anim_vars,
            "#0083",
            "#0084",
            "#0085",
            priority,
        ));
    }

    // Push and register the Root `hkbStateMachineStateInfo` for both Right & Left offset arms.
    seq_patches.push((
        json_path!["#5138", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::SeqPush,
                value: json_typed!(borrowed, [class_indexes[0], class_indexes[2]]),
            },
            priority,
        },
    ));

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let class_index_to_anim_object_map = dashmap::DashMap::new();
    one_patches.par_extend(
        animation
            .anim_objects
            .par_iter()
            .enumerate()
            .map(|(index, name)| {
                let new_anim_object_index = owned_data.next_class_name_attribute();
                class_index_to_anim_object_map.insert(index, new_anim_object_index.clone());
                let one_anim_obj = (
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
                );
                one_anim_obj
            }),
    );

    // $RI
    one_patches.push({
        // $-o+|#%RI+1%|h-|#%RI+1%|ac1|#%RI+1%|null$
        let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects)
            || !flags.contains(FNISAnimFlags::HeadTracking)
            || flags.contains(FNISAnimFlags::AnimatedCameraSet)
        {
            &class_indexes[1]
        } else {
            "#0000"
        };
        // $-o-|#%RI+2%|h+|#%RI+2%|F|#%RI+2%|ac0|#%RI+2%|null$
        let exit_notify_events = if !flags.contains(FNISAnimFlags::AnimObjects)
            || flags.contains(FNISAnimFlags::HeadTracking)
            || flags.contains(FNISAnimFlags::AnimatedCameraReset)
        {
            &class_indexes[2]
        } else {
            "#0000"
        };

        // $-F|#%RI+3%|L|null|#%:FuTransExit%$
        let transition = if matches!(current_phase, FurniturePhase::End(_)) {
            "#0000"
        } else {
            &class_indexes[3] // `#%:FuTransExit` only matched two entries within mt_behavior, both pointing to RI+3.
        };

        // #$-AV|%RI+4%|%RI+7%$
        let generator = if is_empty_anim_vars {
            &class_indexes[7]
        } else {
            &class_indexes[4]
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
                        "transitions": transition,
                        "generator": generator,
                        "name": event_name,
                        "stateId": class_index_0_id, // $1+.fu$
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });
    one_patches.push({
        // #$:AnimObj+&ao$
        let anim_obj_class_index = class_index_to_anim_object_map.get(&0).map(|v| v.clone());
        new_event_property_array_ri1(
            flags,
            anim_obj_class_index.as_deref(),
            &class_indexes[1],
            priority,
        )
    });
    one_patches.push(new_event_property_array_ri2(
        flags,
        current_phase,
        &class_indexes[2],
        priority,
    ));

    // #$RI+3$ hkbStateMachineTransitionInfoArray
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[3].clone()),
            Cow::Borrowed("hkbStateMachineTransitionInfoArray"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[3],
                    "transitions": [
                        {
                            "triggerInterval": {
                                "enterEventId": -1,
                                "exitEventId": -1,
                                "enterTime": 0.0,
                                "exitTime": 0.0,
                            },
                            "initiateInterval": {
                                "enterEventId": -1,
                                "exitEventId": -1,
                                "enterTime": 0.0,
                                "exitTime": 0.0,
                            },
                            "transition": FNIS_BA_BLEND_TRANSITION_5231, // #$:BlendTransition+&bl$
                            "condition": "#0000",
                            "eventId": 152, // IdleChairExitStart

                            // Last index of the sequence. (e.g., len 4 is 4. However,
                            // we use the hash of the last seq state class index here.)
                            "toStateId": end_anim_state_id, // $&fu$
                            "fromNestedStateId": 0,
                            "toNestedStateId": 0,
                            "priority": 0,
                            "flags": "FLAG_DISABLE_CONDITION",
                        }
                    ],
                }),
            },
            priority,
        },
    ));

    // Skip to RI+7 if non use anim_vars
    if !is_empty_anim_vars {
        // RI+4
        one_patches.push((
            vec![
                Cow::Owned(class_indexes[4].clone()),
                Cow::Borrowed("hkbModifierGenerator"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[4],
                        "variableBindingSet": "#0000", // null
                        "userData": 1,
                        "name": "IdleMesmerizes_MG",
                        "modifier": class_indexes[5],
                        "generator": class_indexes[7],
                    }),
                },
                priority,
            },
        ));

        // #RI+5
        one_patches.push({
            let inverse_flags: [bool; 5] = core::array::from_fn(|i| {
                animation
                    .flag_set
                    .anim_vars
                    .get(i)
                    .is_some_and(|v| v.inverse)
            });

            (
                vec![
                    Cow::Owned(class_indexes[5].clone()),
                    Cow::Borrowed("BSIsActiveModifier"),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Pure { op: Op::Add },
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": class_indexes[5],
                            "variableBindingSet": &class_indexes[6],
                            "userData": 2,
                            "name": "bMesmerizeIsActive",
                            "enable": true,
                            "bIsActive0": false,
                            "bInvertActive0": inverse_flags[0],
                            "bIsActive1": false,
                            "bInvertActive1": inverse_flags[1],
                            "bIsActive2": false,
                            "bInvertActive2": inverse_flags[2],
                            "bIsActive3": false,
                            "bInvertActive3": inverse_flags[3],
                            "bIsActive4": false,
                            "bInvertActive4": inverse_flags[4],
                        }),
                    },
                    priority,
                },
            )
        });

        one_patches.push({
            let bindings: Vec<_> = animation
                .flag_set
                .anim_vars
                .iter()
                .enumerate()
                .map(|(i, var)| {
                    simd_json::json_typed!(borrowed, {
                        "memberPath": format!("bIsActive{i}"), // FIXME ?: Is this correct?
                        "variableIndex": format!("$variableID{}$", var.name),  // $&AVI$
                        "bitIndex": -1,
                        "bindingType": "BINDING_TYPE_VARIABLE"
                    })
                })
                .collect();

            (
                vec![
                    Cow::Owned(class_indexes[6].clone()),
                    Cow::Borrowed("hkbVariableBindingSet"),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Pure { op: Op::Add },
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": class_indexes[6],
                            "bindings": bindings,
                            "indexOfBindingToEnable": -1
                        }),
                    },
                    priority,
                },
            )
        });
    }

    // #$RI+7$  hkbClipGenerator
    one_patches.push({
        //  $-F|IdleBlessingKneelEnter|FNISCG_%BI%$
        // The meaning of this template block is:
        // - first: `IdleBlessingKneelEnter`
        // - 2nd: `FNISCG_1`
        // - 3rd: `FNISCG_2`
        // Since the fu syntax appears twice in one list, names overlap, so it's probably unnecessary to make them unique per file.
        let name = match current_phase {
            FurniturePhase::Start => Cow::Borrowed("IdleBlessingKneelEnter"),
            FurniturePhase::Middle(index) | FurniturePhase::End(index) => {
                format!("FNISCG_{index}").into()
            }
        };

        // NOTE: `T` is probably has_triggers
        // $-L|#%RI+8%|a|#%RI+8%|T|#%RI+8%|null$
        let triggers_index = if matches!(current_phase, FurniturePhase::End(_))
            || flags.contains(FNISAnimFlags::Acyclic)
            || !animation.flag_set.triggers.is_empty()
        {
            &class_indexes[8]
        } else {
            "#0000"
        };

        // $-a|MODE_SINGLE_PLAY|MODE_LOOPING$
        let mode = if flags.contains(FNISAnimFlags::Acyclic) {
            "MODE_SINGLE_PLAY"
        } else {
            "MODE_LOOPING"
        };

        (
            vec![
                Cow::Owned(class_indexes[7].clone()),
                Cow::Borrowed("hkbClipGenerator"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[7],
                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": name,
                        "animationName": anim_file, // Animations\\Ffu$
                        "triggers": triggers_index,
                        "cropStartAmountLocalTime": 0.0,
                        "cropEndAmountLocalTime": 0.0,
                        "startTime": 0.0,
                        "playbackSpeed": 1.0,
                        "enforcedDuration": 0.0,
                        "userControlledTimeFraction": 0.0,
                        "animationBindingIndex": -1,
                        "mode": mode,
                        "flags": 0
                    }),
                },
                priority,
            },
        )
    });
    one_patches.push({
        let triggers = new_values_from_triggers(
            flags,
            &animation.flag_set.triggers,
            event_name,
            next_event_name,
            current_phase,
        );

        (
            vec![
                Cow::Owned(class_indexes[8].clone()),
                Cow::Borrowed("hkbClipTriggerArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[8],
                        "triggers": triggers,
                    }),
                },
                priority,
            },
        )
    });

    (one_patches, seq_patches)
}

#[must_use]
fn new_event_property_array_ri1<'a>(
    flags: FNISAnimFlags,
    anim_object_index: Option<&str>,
    class_index: &str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = Vec::new();

    // AnimObjects block (ao)
    if let Some(anim_obj_index) = anim_object_index {
        // 393: AnimObjLoad / 394: AnimObjDraw
        for id in [393, 394] {
            events.push(json_typed!(borrowed, {
                "id": id,
                "payload": anim_obj_index
            }));
        }
    }

    // HeadTracking OFF block (h-)
    if !flags.contains(FNISAnimFlags::HeadTracking) {
        events.push(json_typed!(borrowed, {
            "id": 20, // HeadTrackingOff
            "payload": "#0000"
        }));
    }

    // AnimatedCameraSet block (-ac1)
    if flags.contains(FNISAnimFlags::AnimatedCameraSet) {
        events.push(json_typed!(borrowed, {
            "id": 461, // StartAnimatedCamera
            "payload": "#1978"
        }));
    }

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

#[must_use]
fn new_event_property_array_ri2<'a>(
    flags: FNISAnimFlags,
    current_phase: FurniturePhase,
    class_index: &str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = Vec::new();

    // AnimObjects OFF block (-o-)
    if !flags.contains(FNISAnimFlags::AnimObjects) {
        events.push(json_typed!(borrowed, {
            "id": 165, // AnimObjectUnequip
            "payload": "#0000"
        }));
    }

    // HeadTracking ON block (-h+)
    if flags.contains(FNISAnimFlags::HeadTracking) {
        events.push(json_typed!(borrowed, {
            "id": 18, // HeadTrackingOn
            "payload": "#0000"
        }));
    }

    if matches!(current_phase, FurniturePhase::Start) {
        events.push(json_typed!(borrowed, {
            "id": 14, // IdleChairSitting
            "payload": "#0000"
        }));
    }

    // AnimatedCameraReset block (-ac0)
    if flags.contains(FNISAnimFlags::AnimatedCameraReset) {
        events.push(json_typed!(borrowed, {
            "id": 462, // EndAnimatedCamera
            "payload": "#0000"
        }));
    }

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

#[must_use]
fn new_values_from_triggers<'a>(
    flags: FNISAnimFlags,
    triggers: &[Trigger<'a>],
    current_event_name: &'a str,
    next_event_name: Option<&'a str>,
    current_phase: FurniturePhase,
) -> Vec<Value<'a>> {
    let mut values: Vec<Value> = vec![];
    values.par_extend(triggers.par_iter().map(|Trigger { event, time }| {
        json_typed!(borrowed, {
            "localTime": time, // $&TT$
            "event": {
                "id": format!("$eventID[{event}]$"), // use Nemesis eventID variable. instead of $&TAE$
                "payload": "#0000"
            },
            // When there is no trigger time. The existence of this syntax may imply that the <time> element
            // in the T<event>/<time> syntax is optional.
            "relativeToEndOfClip": false, // FIXME: $&TT-$
            "acyclic": false,
            "isAnnotation": false
        })
    }));

    // `$-L$` <- When Furniture seq is last
    if matches!(current_phase, FurniturePhase::End(_)) {
        // "$AE1fug+&fug$", "5"
        values.push(json_typed!(borrowed, {
            "localTime": -0.05,
            "event": {
                "id": 130, // IdleForceDefaultState
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));
        values.push(json_typed!(borrowed, {
            "localTime": -0.2,
            "event": {
                "id": format!("$eventID[{current_event_name}]$"), // DONE AnimEvent: $AE1fug+&fug$ (Maybe seq last event)
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));
        values.push(json_typed!(borrowed, {
            "localTime": -0.2,
            "event": {
                "id": 5, // IdleFurnitureExit
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));
    }

    // NOTE: Furniture can only have the Acyclic flag set at the first or end.
    // And since next is required, this can only occur at the first.
    if flags.contains(FNISAnimFlags::Acyclic) {
        // If there is no next, then it is last.
        if let Some(next_event_name) = next_event_name {
            values.push(json_typed!(borrowed, {
                "localTime": 0.0,
                "event": {
                    "id": format!("$eventID[{next_event_name}]$"), // next AnimEvent: $AE1fu+1/1$
                    "payload": "#0000"
                },
                "relativeToEndOfClip": false,
                "acyclic": false,
                "isAnnotation": false
            }));
        }
    }

    values
}

/// This variable likely needs to be registered below.
/// - `hkbBehaviorGraphStringData.variableNames`
/// - `hkbVariableValueSet.wordVariableValues`
/// - `hkbBehaviorGraphData.variableInfos`(as [i32])
pub fn new_push_values_seq_patch<'a>(
    values: &[AnimVar<'a>],
    string_data_index: &'static str,
    variable_index: &'static str,
    behavior_graph_index: &'static str,
    priority: usize,
) -> [(json_path::JsonPath<'a>, ValueWithPriority<'a>); 3] {
    [
        (
            json_path![
                string_data_index,
                "hkbBehaviorGraphStringData",
                "variableNames",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: values.iter().map(|value| value.name).collect(),
                },
                priority,
            },
        ),
        (
            json_path![variable_index, "hkbVariableValueSet", "wordVariableValues"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(
                        borrowed,
                        values
                            .par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "value": 0
                                })
                            })
                            .collect::<Vec<_>>()
                    ),
                },
                priority,
            },
        ),
        (
            json_path![
                behavior_graph_index,
                "hkbBehaviorGraphData",
                "variableInfos",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(
                        borrowed,
                        values
                            .par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "role": {
                                        "role": "ROLE_DEFAULT",
                                        "flags": "0"
                                    },
                                    "type": "VARIABLE_TYPE_INT32"
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
