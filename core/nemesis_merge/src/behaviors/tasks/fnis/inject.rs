// SPDX-FileCopyrightText: (C) Monitor221hz
// This is based on the logic of Pandora-Behaviour-Engine-Plus.
// See Unabandon Public License(<repo_root>/resource/xml/templates/LICENSE.md)
use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

fn inject_graph_reference<'a>(
    push_target_index: &'a str,
    patches: &mut (OnePatchMap<'a>, SeqPatchMap<'a>),
    priority: usize,
) {
    let file_stem_str = "";
    let inner_path = "";

    let generator_index = "#0000"; // TODO

    let generator_path = json_path![generator_index, "hkbBehaviorReferenceGenerator"];
    let generator = ValueWithPriority {
        patch: JsonPatch {
            op: OpRangeKind::Pure(Op::Add),
            value: json_typed!(borrowed, {
                "__ptr": generator_index, // TODO:
                "name": file_stem_str, // StringPtr
                "behaviorName": inner_path, // StringPtr
            }),
        },
        priority: 0,
    };

    let hashed_inner_path = {
        use std::hash::{DefaultHasher, Hash as _, Hasher as _};
        let mut hasher = DefaultHasher::new();
        inner_path.hash(&mut hasher);
        (hasher.finish() as i32).to_string()
    };

    // Push root class's seq field
    let state_info_path = json_path![push_target_index, "hkbBehaviorReferenceGenerator", "states"]; // TODO:
    let state_info = ValueWithPriority {
        patch: JsonPatch {
            op: OpRangeKind::Pure(Op::Add),
            value: json_typed!(borrowed, {
                "name": "PN_StateInfo",
                "generator": generator_index,
                "stateId": hashed_inner_path.as_str(),
                "probability": 1.0,
                "enable": true,
            }),
        },
        priority: 0, // TODO
    };

    let (_, seq) = patches;
    seq.insert(state_info_path, state_info);
}
