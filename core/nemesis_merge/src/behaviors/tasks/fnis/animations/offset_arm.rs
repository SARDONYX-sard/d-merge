// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use json_patch::{json_path, JsonPatch, Op, OpRange, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::{animations::FNISAnimation, FNISAnimFlags, FNISAnimType};
use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

#[derive(Debug, Clone)]
pub struct OffsetArmAnimation<'a> {
    pub(crate) template_type: FNISAnimType,
    pub(crate) flags: FNISAnimFlags,

    event_id: &'a str,
    animation_file_path: &'a str,

    anim_object_names: &'a [String],
    pub(crate) next_animation: Option<Box<FNISAnimation<'a>>>,
}

impl<'a> OffsetArmAnimation<'a> {
    pub const fn new(
        template_type: FNISAnimType,
        flags: FNISAnimFlags,
        event_id: &'a str,
        animation_file_path: &'a str,
        anim_object_names: &'a [String],
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
        unique_index: &str,
        priority: usize,
    ) {
        let hashed_anim_path = {
            use std::hash::{DefaultHasher, Hash as _, Hasher as _};
            let mut hasher = DefaultHasher::new();

            self.animation_file_path.hash(&mut hasher);
            (hasher.finish() as i32).to_string()
        };

        let clip_ptr = format!("{unique_index}_clip_generator");

        // Add root class
        let clip_generator_path = json_path!["$offset", "hkbClipGenerator"];
        let clip_generator = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": clip_ptr,
                    "animationName": self.animation_file_path,
                    "playbackSpeed": 1.0,
                    "mode": "MODE_LOOPING",
                }),
            },
            priority,
        };

        // Add root class
        let transition_info_path = json_path!["#4038", "hkbStateMachineTransitionInfo"];
        let transition_info = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "transition": "#0093", // ptr
                    "eventID": self.event_id, // I32<'a>
                    "oStateId": hashed_anim_path.as_str(), // I32<'a>
                    "flags": "FLAG_IS_LOCAL_WILDCARD|FLAG_IS_GLOBAL_WILDCARD|FLAG_DISABLE_CONDITION"
                }),
            },
            priority,
        };

        // Add inner class(This means, doesn't have name attribute(e.g. <name = "#0000">))
        // hkbStateMachineStateInfo
        let state_info = json_typed!(borrowed, {
            "name": format!("{}_StateInfo", self.event_id),
            "probability": 1.0, // f32
            "generator": clip_ptr,
            "stateId": hashed_anim_path.as_str(),
            "enable": true,
            "transitions": "#5111",
        });

        // Push root class's seq field
        // "#5138", "hkbStateMachine", "wildcardTransitions" -> #4038
        let right_arm_path =
            json_path!["#4038", "hkbStateMachineTransitionInfoArray", "transitions"];
        let right_arm = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Seq(OpRange {
                    op: Op::Add,
                    range: 8..9, // intended push(len 9 to 10)
                }),
                value: vec![state_info.clone()].into(),
            },
            priority,
        };

        // Push root class's seq field
        // "#5183", "hkbStateMachine", "wildcardTransitions" -> #5141
        let left_arm_path =
            json_path!["#5141", "hkbStateMachineTransitionInfoArray", "transitions"];
        let left_arm = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Seq(OpRange {
                    op: Op::Add,
                    range: 7..8, // intended push(len 8 to 9)
                }),
                value: vec![state_info].into(),
            },
            priority,
        };

        let (one, seq) = patches;
        one.insert(clip_generator_path, clip_generator);
        one.insert(transition_info_path, transition_info);
        seq.insert(right_arm_path, right_arm);
        seq.insert(left_arm_path, left_arm);
    }
}
