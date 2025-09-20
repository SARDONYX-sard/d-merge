// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::{animations::FNISAnimation, FNISAnimFlags, FNISAnimType};
use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

#[derive(Debug, Clone)]
pub struct FurnitureAnimation<'a> {
    pub(crate) template_type: FNISAnimType,
    pub(crate) flags: FNISAnimFlags,

    event_id: &'a str,
    animation_file_path: &'a str,

    anim_object_names: &'a [String],
    pub(crate) next_animation: Option<Box<FNISAnimation<'a>>>,
}

impl<'a> FurnitureAnimation<'a> {
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

        let binding_index = "#0000"; // TODO

        let binding_path = json_path![binding_index, "hkbVariableBindingSetBinding"];
        let binding = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": binding_index,
                    "bindingType": "0",
                    "bitIndex": "-1",
                    "variableIndex": "1", //bAnimationDriven,
                    "memberPath": "isActive",
                }),
            },
            priority,
        };

        // Add root class
        let idle_furniture_exit_trigger_path = json_path!["", "hkbClipTrigger"];
        let idle_furniture_exit_trigger = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "localTime": -0.2,
                    "relativeToEndOfClip": true,
                    "mode": "MODE_LOOPING",

                    // hkbEventProperty
                    "event": {
                        "__parent": { // hkbEventBase
                            "id": "20"
                        }
                    },
                }),
            },
            priority,
        };

        // Add root class
        let clip_ptr = format!("{unique_index}_clip_generator");
        let furniture_state_path = json_path!["#4038", "hkbStateMachine"];
        let furniture_state = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": clip_ptr,
                    "name": format!("{}_Behavior", self.event_id), // ptr
                    "binding": [binding]
                }),
            },
            priority,
        };

        // Add root class
        let variable_binding_name = format!("{unique_index}_variable_binding_set");
        let variable_binding_path = json_path!["", "hkbStateMachine"];
        let variable_binding = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": variable_binding_name,
                }),
            },
            priority,
        };

        let (one, _seq) = patches;
        one.insert(variable_binding_path, variable_binding);
        one.insert(furniture_state_path, furniture_state);
    }
}
