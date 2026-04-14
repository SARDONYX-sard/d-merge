use std::borrow::Cow;

use json_patch::{Action, JsonPatch, ValueWithPriority, json_path};
use rayon::prelude::*;
use simd_json::borrowed::Value;

use crate::behaviors::{
    fnis::patch_gen::alternate::generated_group_table::ALT_GROUPS,
    tasks::fnis::patch_gen::generated_behaviors::{BehaviorEntry, DEFAULT_FEMALE},
};

/// This variable likely needs to be registered below.
/// - `hkbBehaviorGraphStringData.variableNames`
/// - `hkbVariableValueSet.wordVariableValues`
/// - `hkbBehaviorGraphData.variableInfos`(as [i32])
///
/// # Target template
/// `0_master`
pub fn new_push_alt_anim_values_seq_patch<'a>(
    priority: usize,
) -> [(json_path::JsonPath<'a>, ValueWithPriority<'a>); 3] {
    let BehaviorEntry {
        master_string_data_index: string_data_index,
        master_behavior_graph_index: behavior_graph_index,
        ..
    } = DEFAULT_FEMALE;

    let keys = {
        let mut sorted_entries: Vec<_> = ALT_GROUPS
            .into_iter()
            .map(|(&key, group)| (group.id, key))
            .collect();
        sorted_entries.par_sort_by_key(|&(id, _)| id);

        let mut keys: Vec<Value> = sorted_entries
            .iter()
            .map(|&(_, key)| Value::String(Cow::Owned(format!("FNISaa{key}"))))
            .collect();

        keys.par_extend(
            sorted_entries
                .par_iter()
                // e.g., FNISaa_1hmeqp_crc
                .map(|(_, key)| Value::String(Cow::Owned(format!("fNISaa{}_crc", *key))))
                .chain(rayon::iter::once(Value::String(Cow::Borrowed(
                    "FNISaa_crc",
                )))),
        );
        keys
    };

    [
        (
            json_path!["#0107", "hkbVariableValueSet", "wordVariableValues"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(
                        borrowed,
                        keys.par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "value": 0
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
                        keys.par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "role": {
                                        "role": "ROLE_DEFAULT",
                                        "flags": "0"
                                    },
                                    "type": "VARIABLE_TYPE_INT32"
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
                string_data_index,
                "hkbBehaviorGraphStringData",
                "variableNames",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: keys.into(),
                },
                priority,
            },
        ),
    ]
}
