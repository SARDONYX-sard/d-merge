// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use json_patch::{json_path, JsonPatch, Op, OpRange, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::list_parser::combinator::anim_types::FNISAnimType;
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::FNISAnimFlags;
use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

#[derive(Debug, Clone, Hash)]
pub struct FurnitureAnimation<'a, 'b> {
    pub(crate) template_type: FNISAnimType,
    pub(crate) flags: FNISAnimFlags,

    event_id: &'a str,
    animation_file_path: &'a str,

    anim_object_names: &'b [&'a str],
    pub(crate) next_animation: Option<Box<FurnitureAnimation<'a, 'b>>>,
}

impl<'a, 'b> FurnitureAnimation<'a, 'b> {
    pub const fn new(
        template_type: FNISAnimType,
        flags: FNISAnimFlags,
        event_id: &'a str,
        animation_file_path: &'a str,
        anim_object_names: &'b [&'a str],
    ) -> Self {
        Self {
            template_type,
            flags,
            event_id,
            animation_file_path,
            anim_object_names,
            next_animation: None,
        }
    }

    // - `state_info_id`: hkbStateMachineStateInfo root class name att r
    // - `clip_id`: `hkbClipGenerator.triggers`(Pointer). It's `hkbClipTriggerArray`
    fn build_flags(
        patches: &(OnePatchMap<'a>, SeqPatchMap<'a>),
        priority: usize,
        mod_code: &'a str,
        state_info_id: &'a str,
    ) {
        // TODO: Call basic anim build_flags

        let (_, seq) = patches;

        // if self.flags.contains(FNISAnimFlags::SequenceFinish) {
        //     return;
        // }

        // Push headtracking event to `event(hkbStateMachineEventPropertyArray)`.
        seq.insert(
            json_path![
                state_info_id,
                "hkbStateMachineEventPropertyArray",
                "exitNotifyEvents",
                "events"
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(json_patch::OpRange {
                        op: Op::Add,
                        range: 9999..9999, // FIXME?: intended push to the last.
                    }),
                    // hkbEventProperty
                    value: json_typed!(borrowed, { "id": "18" }),
                },
                priority,
            },
        );

        // push `idleFurnitureExitTrigger` to triggers
        seq.insert(
            json_path![mod_code, "hkbClipTriggerArray", "triggers"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(json_patch::OpRange {
                        op: Op::Add,
                        range: 9999..9999, // FIXME?: intended push to the last.
                    }),
                    // `idleFurnitureExitTrigger` patch
                    value: json_typed!(borrowed, {
                        "localTime": -0.2,
                        "relativeToEndOfClip": true,
                        // hkbEventProperty
                        "event": {
                            "id": "5"
                        },
                    }),
                },
                priority,
            },
        );
    }

    /// Push patch to `meshes/actors/character/behaviors/mt_behavior.xml` (from hkxcmd index rule)
    ///
    /// `unique_index`: unique name attribute(e.g. `$aaaa`)
    fn build_behavior(
        &self,
        patches: &(OnePatchMap, SeqPatchMap),
        mod_code: &str,
        priority: usize,
    ) {
        let (one, seq) = patches;

        // e.g. `IdleForceDefaultState`
        let event_name = "";
        let hashed_anim_path = hash(self.animation_file_path).to_string();
        let state_id = hashed_anim_path.as_str();

        let furniture_behavior_state_unique_index = format!("#FNIS_{mod_code}$0");
        let binding_set_unique_index = format!("#FNIS_{mod_code}$1");
        let group_state_info_unique_index = format!("#FNIS_{mod_code}$2");
        let enter_clip_generator_unique_index = format!("#FNIS_{mod_code}$3");

        let enter_event_array_unique_index = format!("#FNIS_{mod_code}$4");
        let exit_event_array_unique_index = format!("#FNIS_{mod_code}$5");

        let enter_state_info_unique_index = format!("#FNIS_{mod_code}$6");
        let exit_transition_array_unique_index = format!("#FNIS_{mod_code}$7");
        let transition_effect_unique_index = format!("#FNIS_{mod_code}$8");

        one.insert(
            vec![
                furniture_behavior_state_unique_index.clone().into(),
                "hkbStateMachine".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": furniture_behavior_state_unique_index, // e.g. "#mod_code$1"
                        "name": format!("{}_Behavior", self.event_id), // StringPtr

                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": "EAS_ChairEU_SmallTableSelector",
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
                        "states": [enter_state_info_unique_index], // Vec<Pointer>
                        "wildcardTransitions": "#0000"
                    }),
                },
                priority,
            },
        );

        one.insert(
            vec![
                binding_set_unique_index.clone().into(),
                "hkbVariableBindingSet".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": binding_set_unique_index,
                        "bindings": [
                            // animationDrivenBinding
                            {
                                "memberPath": "isActive",
                                "variableIndex": 1, // <- `hkbBehaviorGraphStringData.eventNames`[`bAnimationDriven`] index
                                "bitIndex": -1,
                                "bindingType": "BINDING_TYPE_VARIABLE"
                            }
                        ],
                        "indexOfBindingToEnable": -1
                    }),
                },
                priority,
            },
        );

        // TODO: self.flags.contains(FNISAnimFlags::SequenceStart)
        let is_seq_start = false;
        let transitions = if is_seq_start {
            let mut last_anim = None;
            while let Some(next_anim) = &self.next_animation {
                last_anim = Some(next_anim);
            }
            let state_id = hash(last_anim);
            json_typed!(borrowed, [{
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
                "transition": transition_effect_unique_index,
                "condition": "#0000",
                "eventId": 152, // <- index
                "toStateId": state_id, // <- furniture id
                "fromNestedStateId": 0,
                "toNestedStateId": state_id,
                "priority": 0,
                "flags": "FLAG_TO_NESTED_STATE_ID_IS_VALID|FLAG_IS_LOCAL_WILDCARD"
            }])
        } else {
            json_typed!(borrowed, [])
        };

        one.insert(
            vec![
                exit_transition_array_unique_index.clone().into(),
                "hkbStateMachineTransitionInfoArray".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                            "__ptr": exit_transition_array_unique_index,
                            "transitions": transitions,
                        }
                    ),
                },
                priority,
            },
        );

        one.insert(
            vec![
                group_state_info_unique_index.clone().into(),
                "hkbStateMachineStateInfo".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                            "__ptr": group_state_info_unique_index,
                            "variableBindingSet": binding_set_unique_index,
                            "listeners": [],
                            "enterNotifyEvents": "#0000",
                            "exitNotifyEvents": "#0000",
                            "transitions": exit_transition_array_unique_index,
                            "generator": furniture_behavior_state_unique_index,
                            "name": format!("{mod_code}_{event_name}_State"),
                            "stateId": state_id,
                            "probability": 1.0,
                            "enable": true
                        }
                    ),
                },
                priority,
            },
        );

        let anim_path = format!("Animations\\{}.hkx", self.animation_file_path); // FIXME:?
        one.insert(
            vec![
                enter_clip_generator_unique_index.clone().into(),
                "hkbClipGenerator".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": enter_clip_generator_unique_index,
                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": "IdleBlessingKneelEnter",
                        "animationName": anim_path,
                        "triggers": "#0000",
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
        );

        seq.insert(
            vec![
                enter_event_array_unique_index.clone().into(),
                "hkbStateMachineEventPropertyArray".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                            "__ptr": enter_event_array_unique_index,
                            "events": [
                                {
                                    "id": 20,
                                    "payload": "#0000"
                                },
                            ]
                        }
                    ),
                },
                priority,
            },
        );
        seq.insert(
            vec![
                exit_event_array_unique_index.clone().into(),
                "hkbStateMachineEventPropertyArray".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                            "__ptr": enter_event_array_unique_index,
                            "events": [
                                {
                                    "id": 14,
                                    "payload": "#0000"
                                },
                            ]
                        }
                    ),
                },
                priority,
            },
        );

        one.insert(
            vec![
                enter_state_info_unique_index.clone().into(),
                "hkbStateMachineStateInfo".into(),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": enter_state_info_unique_index,
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_event_array_unique_index,
                        "exitNotifyEvents": exit_event_array_unique_index,
                        "transitions": exit_transition_array_unique_index,
                        "generator": enter_clip_generator_unique_index,
                        "name": format!("{event_name}_StateInfo"),
                        "stateId": state_id,
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        );

        one.insert(
            vec![
                transition_effect_unique_index.clone().into(),
                "hkbBlendingTransitionEffect".into()
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": transition_effect_unique_index,
                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": "EASChair_BlendingTransitionEffect",
                        "selfTransitionMode": "SELF_TRANSITION_MODE_CONTINUE_IF_CYCLIC_BLEND_IF_ACYCLIC",
                        "eventMode": "EVENT_MODE_DEFAULT",
                        "duration": 0.5,
                        "toGeneratorStartTimeFraction": 0.0,
                        "flags": "0",
                        "endMode": "END_MODE_NONE",
                        "blendCurve": "BLEND_CURVE_SMOOTH"
                    }),
                },
                priority,
            },
        );

        // Root TransitionInfoArray
        one.insert(
            json_path!["#0089", "hkbStateMachineTransitionInfoArray", "transitions"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 9998..9999,
                    }),
                    value: json_typed!(borrowed,
                        [
                            {
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
                                "transition": transition_effect_unique_index,
                                "condition": "#0000",
                                "eventId": format!("$eventID[{event_name}]$"),
                                "toStateId": 4, // <- furniture id
                                "fromNestedStateId": 0,
                                "toNestedStateId": state_id,
                                "priority": 0,
                                "flags": "FLAG_TO_NESTED_STATE_ID_IS_VALID|FLAG_IS_LOCAL_WILDCARD"
                            }
                        ]
                    ),
                },
                priority,
            },
        );

        one.insert(
            json_path!["#4002", "hkbStateMachine", "states"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 9998..9999,
                    }),
                    value: json_typed!(borrowed, group_state_info_unique_index),
                },
                priority,
            },
        );
    }
}

fn hash(t: impl std::hash::Hash) -> i32 {
    use std::hash::{DefaultHasher, Hasher as _};
    let mut hasher = DefaultHasher::new();

    t.hash(&mut hasher);
    hasher.finish() as i32
}
