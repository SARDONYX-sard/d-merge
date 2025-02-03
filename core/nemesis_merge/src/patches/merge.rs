use crate::aliases::{MergedPatchMap, SortedPatchMap, TemplatePatchMap};
use crate::errors::Result;
use crate::paths::parse::get_nemesis_id;
use json_patch::{JsonPatch, Op, OpRange, OpRangeKind};
use rayon::prelude::*;
use simd_json::{
    base::ValueTryAsArrayMut as _, borrowed::Value, derived::ValueTryIntoArray, StaticNode,
};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn paths_to_ids(paths: &[PathBuf]) -> Vec<String> {
    paths
        .par_iter()
        .filter_map(|path| get_nemesis_id(path).ok())
        .collect()
}

pub fn merge_patches<'p>(
    patches: TemplatePatchMap<'p>,
    ids: &[String],
) -> Result<MergedPatchMap<'p>> {
    let merged_patches = MergedPatchMap::new();

    patches.into_par_iter().for_each(|idx_map| {
        let (template_name, patch_idx_map) = idx_map;

        patch_idx_map.into_par_iter().for_each(|patch| {
            let (_index, mut patches) = patch;

            let sorted_patches: Vec<SortedPatchMap<'_>> =
                ids.iter().filter_map(|id| patches.remove(id)).collect();

            let mut merged_result = HashMap::new();
            for patch_map in sorted_patches {
                merge_json_patches(&mut merged_result, patch_map);
            }

            merged_patches
                .entry(template_name.clone())
                .or_default()
                .par_extend(merged_result);
        });
    });

    Ok(merged_patches)
}

fn merge_json_patches<'a>(base: &mut SortedPatchMap<'a>, additional: SortedPatchMap<'a>) {
    for (key, value) in additional {
        if let Some(base_mut) = base.get_mut(&key) {
            merge_inner(base_mut, value);
        } else {
            base.insert(key, value);
        }
    }
}

/// - `base`: low priority patch
/// - `patch`: high priority patch
fn merge_inner<'a>(base: &mut JsonPatch<'a>, patch: JsonPatch<'a>) {
    // 1 separate range and non-range
    let JsonPatch {
        op: base_op,
        value: base_value,
    } = base;

    let JsonPatch {
        op: patch_op,
        value: patch_value,
    } = patch;

    match &patch_op {
        OpRangeKind::Seq(patch_op_range) => {
            match base_op {
                OpRangeKind::Seq(base_op_range) => {
                    let OpRange {
                        op: base_op,
                        range: base_range,
                    } = base_op_range;

                    let OpRange {
                        op: patch_op,
                        range: patch_range,
                    } = patch_op_range;

                    let equal_replace =
                        *base_op == Op::Remove && *patch_op == Op::Add && base_range == patch_range;

                    if equal_replace {
                        *base_op = Op::Replace;
                        *base_value = patch_value;
                    }
                }
                OpRangeKind::Discrete(_op_ranges) => {}
                OpRangeKind::Pure(_) => {} // TODO: error occurred
            }
        }
        OpRangeKind::Discrete(_op_ranges) => {}
        OpRangeKind::Pure(op) => match op {
            Op::Add => {
                if let Ok(base_arr) = base.value.try_as_array_mut() {
                    if let Ok(additional_arr) = patch_value.try_into_array() {
                        base_arr.extend(additional_arr);
                    }
                }
            }
            Op::Replace => {
                base.value = patch_value;
            }
            Op::Remove => {
                remove(base);
            }
        },
    }
}

fn remove(base: &mut JsonPatch<'_>) {
    match &mut base.value {
        Value::Object(map) => {
            map.clear();
        }
        Value::String(string) => {
            *string = "".into();
        }

        // to default
        Value::Static(pure_value) => match pure_value {
            StaticNode::I64(int) => {
                *int = 0;
            }
            StaticNode::U64(uint) => {
                *uint = 0;
            }
            StaticNode::F64(float) => {
                *float = 0.0;
            }
            StaticNode::Bool(boolean) => {
                *boolean = false;
            }
            StaticNode::Null => {}
        },
        Value::Array(_) => {}
    }
}

// Complete
// - range:
//    - add
//
// TODO
// - range:
//    - remove
//    - replace
// - one field
//    - replace
//    - add (maybe mod author's miss)
//    - remove
#[cfg(test)]
mod tests {
    use super::*;
    use json_patch::{json_path, JsonPatch, Op, OpRange, OpRangeKind};
    use simd_json::json_typed;

    #[test]
    fn test_merge_add_json_patches() {
        let mut base_hash_map = HashMap::new();

        base_hash_map.insert(
            json_path!["#0009", "hkbProjectStringData", "characterFilenames"],
            JsonPatch {
                op: OpRangeKind::Seq(OpRange {
                    op: Op::Add,
                    range: 1656..1657,
                }),
                value: json_typed!(borrowed, "Animations\\1hm_UnsheatheAttack.hkx"),
            },
        );

        let mut add_hash_map = HashMap::new();

        add_hash_map.insert(
            json_path!["#0009", "hkbProjectStringData", "characterFilenames"],
            JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Add,
                    range: 1656..1657,
                }),
                value: json_typed!(borrowed, "Animations\\MomoAJ\\mt_jumpfallback.hkx"),
            },
        );

        merge_json_patches(&mut base_hash_map, add_hash_map);

        let mut expected = HashMap::new();
        let json_path = json_path!["#0029", "hkbCharacterStringData", "animationNames",];

        expected.insert(
            json_path,
            JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Add,
                    range: 1656..1658,
                }),

                value: json_typed!(
                    borrowed,
                    [
                        "Animations\\1hm_UnsheatheAttack.hkx",
                        "Animations\\MomoAJ\\mt_jumpfallback.hkx"
                    ]
                ),
            },
        );

        assert_eq!(base_hash_map, expected);
    }

    #[test]
    fn test_merge_replace_json_patches() {
        let mut base = HashMap::new();
        let json_path = json_path!["#0010", "hpathkbProjectData", "stringData"];

        base.insert(
            json_path,
            JsonPatch {
                op: OpRangeKind::Pure(Op::Replace),
                value: "$id".into(),
            },
        );

        let mut additional = HashMap::new();
        let json_path = json_path!["#0010", "hpathkbProjectData", "stringData"];
        additional.insert(
            json_path.clone(),
            JsonPatch {
                op: OpRangeKind::Pure(Op::Replace),
                value: "$id2".into(),
            },
        );

        merge_json_patches(&mut base, additional.clone());
        assert_eq!(base, additional);
    }

    #[test]
    fn test_seq_diff_op_json_patches() {
        let json_path = json_path!["#0009", "hkbCharacterStringData", "characterFilenames",];
        let mut base_hash_map = HashMap::new();

        base_hash_map.insert(
            json_path.clone(),
            JsonPatch {
                op: OpRangeKind::Seq(OpRange {
                    op: Op::Remove,
                    range: 1656..1657,
                }),
                value: json_typed!(borrowed, null),
            },
        );

        let mut add_hash_map = HashMap::new();

        add_hash_map.insert(
            json_path.clone(),
            JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Add,
                    range: 1656..1657,
                }),
                value: json_typed!(borrowed, ["Animations\\MomoAJ\\mt_jumpfallback.hkx"]),
            },
        );

        merge_json_patches(&mut base_hash_map, add_hash_map);

        let mut expected = HashMap::new();

        expected.insert(
            json_path,
            JsonPatch {
                op: OpRangeKind::Seq(json_patch::OpRange {
                    op: Op::Replace,
                    range: 1656..1657,
                }),

                value: json_typed!(borrowed, ["Animations\\MomoAJ\\mt_jumpfallback.hkx"]),
            },
        );

        assert_eq!(base_hash_map, expected);
    }
}
