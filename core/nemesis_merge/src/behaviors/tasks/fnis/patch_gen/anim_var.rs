use json_patch::{json_path, Action, JsonPatch, ValueWithPriority};
use rayon::prelude::*;

use crate::behaviors::tasks::fnis::{
    collect::owned::OwnedFnisInjection,
    list_parser::combinator::anim_var::{AnimVar, Value},
};

/// This variable likely needs to be registered below to `0_master.xml`.
/// - `hkbBehaviorGraphStringData.variableNames`
/// - `hkbVariableValueSet.wordVariableValues`
/// - `hkbBehaviorGraphData.variableInfos`(as [i32])
pub fn new_push_anim_vars_patch<'a>(
    values: &[AnimVar<'a>],
    owned_data: &'a OwnedFnisInjection,
) -> [(json_path::JsonPath<'a>, ValueWithPriority<'a>); 3] {
    let string_data_index = owned_data.behavior_entry.master_string_data_index;
    let variable_index = owned_data.behavior_entry.master_value_set_index;
    let behavior_graph_index = owned_data.behavior_entry.master_behavior_graph_index;
    let priority = owned_data.priority;

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
                            .map(|anim_var| {
                                let value = match anim_var.value {
                                    Value::Bool(b) => {
                                        if b {
                                            1
                                        } else {
                                            0
                                        }
                                    }
                                    Value::Int32(v) => v,
                                    Value::Real(v) => v as i32,
                                };

                                simd_json::json_typed!(borrowed, {
                                    "value": value
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
                            .map(|anim_var| {
                                let value_type = match anim_var.value {
                                    Value::Bool(_) => "VARIABLE_TYPE_BOOL",
                                    Value::Int32(_) => "VARIABLE_TYPE_INT32",
                                    Value::Real(_) => "VARIABLE_TYPE_REAL",
                                };

                                simd_json::json_typed!(borrowed, {
                                    "role": {
                                        "role": "ROLE_DEFAULT",
                                        "flags": "0"
                                    },
                                    "type": value_type
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
