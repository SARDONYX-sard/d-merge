mod add;
mod basic;
mod furniture;
mod offset_arm;

use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::{
    fnis::collect::owned::OwnedFnisInjection,
    patches::types::{OnePatchMap, SeqPatchMap},
};

/// For Seq patch
pub(crate) const PUSH_OP: OpRangeKind = OpRangeKind::Seq(json_patch::OpRange {
    op: Op::Add,
    range: 9998..9999,
});

/// Register a mod's root behavior (`behaviors\FNIS_<namespace>_Behavior.hkx`)
/// into `meshes\actors\character\behaviors\0_master.xml`.
///
/// - `behavior_id`: Unique identifier for the behavior (e.g., `#<namespace>${index}`).
/// - `behavior_path`: Path to the behavior file used in `hkbBehaviorReferenceGenerator.behavior_name`.
pub fn push_mod_root_behavior<'a>(
    patches: &(OnePatchMap<'a>, SeqPatchMap<'a>),

    owned_data: &'a OwnedFnisInjection,
) {
    let (one, seq) = patches;

    let namespace = owned_data.namespace.as_str();
    let priority = owned_data.priority;
    let behavior_path = owned_data.behavior_path.as_str();
    let new_root_behavior_index = owned_data.next_class_name_attribute();

    one.insert(
        vec![
            Cow::Owned(new_root_behavior_index.clone()),
            Cow::Borrowed("hkbBehaviorReferenceGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": new_root_behavior_index,
                    "variableBindingSet": "#0000", // null
                    "userData": 0,

                    // NOTE: FNIS_ROOT_BFR{index}: In FNIS, it's actually the ordering index.
                    // but here we use priority instead.
                    "name": format!("FNIS_ROOT_BFR_{namespace}_{priority}"), // StringPtr
                    "behaviorName": behavior_path, // StringPtr
                }),
            },
            priority,
        },
    );

    seq.insert(
        json_path!["#0340", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, new_root_behavior_index),
            },
            priority,
        },
    );
}
