// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use json_patch::{json_path, JsonPatch, Op, OpRange, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::{
    animations::FNISAnimation, list_parser::flags::FNISAnimFlags, FNISAnimType,
};
use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

#[derive(Debug, Clone, Hash)]
pub struct OffsetArmAnimation<'a, 'b> {
    pub(crate) template_type: FNISAnimType,
    pub(crate) flags: FNISAnimFlags,

    event_id: &'a str,
    animation_file_path: &'a str,

    anim_object_names: &'b [&'a str],
    pub(crate) next_animation: Option<Box<FNISAnimation<'a, 'b>>>,
}

impl<'a, 'b> OffsetArmAnimation<'a, 'b> {
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

    /// Push patch to `meshes/actors/character/behaviors/mt_behavior.xml` (from hkxcmd index rule)
    ///
    /// `unique_index`: unique name attribute(e.g. `$aaaa`)
    fn build_behavior(
        &self,
        patches: &mut (OnePatchMap, SeqPatchMap),
        mod_code: &str,
        priority: usize,
    ) {
        let event_name = "";

        let anim_path = format!("Animations\\{}.hkx", self.animation_file_path);

        let hashed_anim_path = {
            use std::hash::{DefaultHasher, Hash as _, Hasher as _};
            let mut hasher = DefaultHasher::new();

            self.animation_file_path.hash(&mut hasher);
            (hasher.finish() as i32).to_string()
        };
        let state_id = hashed_anim_path.as_str();

        let mut index = 0;
        let clip_generator_unique_index = format!("#FNIS_{mod_code}${index}");
        index += 1;
        let arm_state_info_unique_index = format!("#FNIS_{mod_code}${index}");

        let (one, seq) = patches;

        {
            one.insert(
                vec![
                    clip_generator_unique_index.clone().into(),
                    "hkbClipGenerator".into(),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Pure(Op::Add),
                        value: json_typed!(borrowed, {
                            "__ptr": clip_generator_unique_index,
                            "variableBindingSet": "#0000",
                            "userData": 0,
                            "name": format!("{mod_code}_{event_name}_Clip"),
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

            one.insert(
                vec![
                    arm_state_info_unique_index.clone().into(),
                    "hkbStateMachineStateInfo".into(),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Pure(Op::Add),
                        value: json_typed!(borrowed, {
                            "__ptr": arm_state_info_unique_index,
                            "variableBindingSet": "#0000",
                            "listeners": [],
                            "enterNotifyEvents": "#0000",
                            "exitNotifyEvents": "#0000",
                            "transitions": "#5111",
                            "generator": clip_generator_unique_index,
                            "name": format!("{event_name}_StateInfo"),
                            "stateId": state_id,
                            "probability": 1.0,
                            "enable": true
                        }),
                    },
                    priority,
                },
            );

            // right arm
            seq.insert(
                json_path!["#5138", "hkbStateMachine", "states"],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Seq(OpRange {
                            op: Op::Add,
                            range: 9998..9999, // Push
                        }),
                        value: json_typed!(borrowed, [arm_state_info_unique_index]),
                    },
                    priority,
                },
            );
            // left arm
            seq.insert(
                json_path!["#5141", "hkbStateMachine", "states"],
                ValueWithPriority {
                    patch: JsonPatch {
                        op: OpRangeKind::Seq(OpRange {
                            op: Op::Add,
                            range: 9998..9999, // Push
                        }),
                        value: json_typed!(borrowed, [arm_state_info_unique_index]),
                    },
                    priority,
                },
            );
        }
        ////////////////////////////////////////////////////////////////////////////////////////////////////////////////

        let state_info = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
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

                    "transition": "#0093",
                    "condition": "#0000",
                    "eventId": format!("$eventID[{event_name}]$"),
                    "toStateId": state_id,
                    "fromNestedStateId": 0,
                    "toNestedStateId": 0,
                    "priority": 0,
                    "flags": "FLAG_TO_NESTED_STATE_ID_IS_VALID|FLAG_DISABLE_CONDITION"
                }),
            },
            priority,
        };

        // "#5138", "hkbStateMachine", "wildcardTransitions"(Pointer) -> #4038
        seq.insert(
            json_path!["#4038", "hkbStateMachineTransitionInfoArray", "transitions"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 9998..9999, // Intended push
                    }),
                    value: json_typed!(borrowed, [state_info]),
                },
                priority,
            },
        );

        // "#5183", "hkbStateMachine", "wildcardTransitions" -> #5141
        seq.insert(
            json_path!["#5141", "hkbStateMachineTransitionInfoArray", "transitions"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 9998..9999, // Intended push
                    }),
                    value: json_typed!(borrowed, [state_info]),
                },
                priority,
            },
        );

        // TODO: call BasicAnimation.build_flags here.
    }
}
