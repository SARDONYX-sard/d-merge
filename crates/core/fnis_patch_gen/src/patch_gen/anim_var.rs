use fnis_list::combinator::anim_var::{AnimVar, Value};
use json_patch::{ValueWithPriority, json_path, json_value};
use rayon::prelude::*;

use crate::{collect::owned::OwnedFnisInjection, patch_gen::new_seq_push};

/// This variable likely needs to be registered below to `0_master.xml`.
/// - `hkbBehaviorGraphStringData.variableNames`
/// - `hkbVariableValueSet.wordVariableValues`
/// - `hkbBehaviorGraphData.variableInfos`(as [i32])
pub(super) fn new_push_anim_vars_patch<'a>(
    values: &[AnimVar<'a>],
    owned_data: &'a OwnedFnisInjection,
) -> [(json_path::JsonPath<'a>, ValueWithPriority<'a>); 3] {
    let string_data_index = owned_data.behavior_entry.master_string_data_index;
    let variable_index = owned_data.behavior_entry.master_value_set_index;
    let behavior_graph_index = owned_data.behavior_entry.master_behavior_graph_index;
    let priority = owned_data.priority;

    [
        new_seq_push(
            json_path![string_data_index, "hkbBehaviorGraphStringData", "variableNames",],
            priority,
            values.iter().map(|value| value.name).collect(),
        ),
        new_seq_push(
            json_path![variable_index, "hkbVariableValueSet", "wordVariableValues"],
            priority,
            json_value!(
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

                        json_value!({
                            "value": value
                        })
                    })
                    .collect::<Vec<_>>()
            ),
        ),
        new_seq_push(
            json_path![behavior_graph_index, "hkbBehaviorGraphData", "variableInfos",],
            priority,
            json_value!(
                values
                    .par_iter()
                    .map(|anim_var| {
                        let value_type = match anim_var.value {
                            Value::Bool(_) => "VARIABLE_TYPE_BOOL",
                            Value::Int32(_) => "VARIABLE_TYPE_INT32",
                            Value::Real(_) => "VARIABLE_TYPE_REAL",
                        };

                        json_value!({
                            "role": {
                                "role": "ROLE_DEFAULT",
                                "flags": "0"
                            },
                            "type": value_type
                        })
                    })
                    .collect::<Vec<_>>()
            ),
        ),
    ]
}
