mod add;
mod basic;
mod furniture;
mod offset_arm;

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use simd_json::json_typed;

use crate::behaviors::tasks::patches::types::{OnePatchMap, SeqPatchMap};

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
pub fn register_mod_root_behavior<'a>(
    patches: &(OnePatchMap<'a>, SeqPatchMap<'a>),
    priority: usize,

    behavior_id: &'a str,
    behavior_path: &'a str,
) {
    let (one, seq) = patches;

    // NOTE: FNIS_ROOT_BFR{index}: In FNIS, it's actually the ordering index.
    // but here we use priority instead.
    one.insert(
        json_path![behavior_id, "hkbBehaviorReferenceGenerator"],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": behavior_id,
                    "variableBindingSet": "#0000", // null
                    "userData": 0,
                    "name": format!("FNIS_ROOT_BFR{priority}"), // StringPtr
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
                value: json_typed!(borrowed, behavior_id),
            },
            priority,
        },
    );
}
