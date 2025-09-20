// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

/// - `push_target_index`: `hkbStateMachine` root class name attr. (e.g. `#0010`)
///
/// - `generator_id`: unique name attribute. (e.g. `$fnis_3`)
/// - `generator_file_stem`: `hkbBehaviorReferenceGenerator`.`name`. (e.g. `dummy`)
/// - `inner_path`: `hkbBehaviorReferenceGenerator`.`behavior_name`. (e.g. `Animation\FNIS_Mod\dummy.hkx`)
fn inject_graph_reference<'a>(
    push_target_index: &'a str,
    patches: &mut (OnePatchMap<'a>, SeqPatchMap<'a>),
    priority: usize,

    generator_id: &'a str,
    generator_file_stem: &'a str,
    inner_path: &'a str,
) {
    let (one, seq) = patches;

    {
        // hkbBehaviorReferenceGenerator (Push as root class)
        let generator = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": generator_id,
                    "name": generator_file_stem, // StringPtr
                    "behaviorName": inner_path, // StringPtr
                }),
            },
            priority,
        };

        let generator_path = json_path![generator_id, "hkbBehaviorReferenceGenerator"];
        one.insert(generator_path, generator);
    }

    {
        let hashed_inner_path_id = {
            use std::hash::{DefaultHasher, Hash as _, Hasher as _};
            let mut hasher = DefaultHasher::new();
            inner_path.hash(&mut hasher);
            (hasher.finish() as i32).to_string()
        };

        // hkbStateMachineStateInfo
        let state_info = ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "name": "PN_StateInfo", // StringPtr
                    "generator": generator_id, // ptr
                    "stateId": hashed_inner_path_id, // I32<'a>
                    "probability": 1.0, // f32
                    "enable": true,
                }),
            },
            priority,
        };

        let state_info_path = json_path![push_target_index, "hkbStateMachine", "states"];
        seq.insert(state_info_path, state_info);
    }
}
