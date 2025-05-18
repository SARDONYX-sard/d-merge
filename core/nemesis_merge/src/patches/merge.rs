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
    // new merged patches
    let merged_patches = MergedPatchMap::new();

    // patch loop
    patches
        .into_par_iter()
        .for_each(|(template_name, patch_idx_map)| {
            // index loop
            patch_idx_map.into_par_iter().for_each(|patch| {
                let (_index, mut patches) = patch;

                // Priority sorting by id and path from GUI
                let sorted_patches: Vec<SortedPatchMap<'_>> =
                    ids.iter().filter_map(|id| patches.remove(id)).collect();

                // Merge json patches
                let mut merged_result = HashMap::new();
                for patch_map in sorted_patches {
                    merge_json_patches(&mut merged_result, patch_map);
                }

                // Insert merged patches
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
        // Same path is mutation.
        // Todo: Except for three patterns in hash key
        // conflict
        // - (non conflict) e.g. Remove: 1..5, Add: 10..11 -> Remove:1..5, Add: 10..11
        // - (patch start) e.g. Remove: 1..5, Add: 4..11 -> Remove:1..3, Add: 4..11
        // - (base end) e.g. Remove: 10..12, Add: 4..10 -> Add: 4..10, Remove:11..12
        if let Some(base_mut) = base.get_mut(&key) {
            merge_inner(base_mut, value);
        } else {
            // Todo: Add three patterns
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

                    let is_same_start = base_range.start == patch_range.start;

                    match (&base_op, &patch_op) {
                        (Op::Add, Op::Add) => {
                            // Add patch
                            // e.g. Add:  1..5,  Add: 10..11 -> Add: 1..5,  Add: 10..11
                            // e.g. Add:  1..5,  Add:  4..11 -> Add: 1..12
                            // e.g. Add:  1..20(+19), Add: 11..15(+4) -> Add: 1..24(+23)
                            // e.g. Add: 10..12, Add:  4..10 -> Add: 4..10, Add: 11..12
                            if is_same_start {
                                let _ =
                                    seq_add(base_range, base_value, patch_range.len(), patch_value);
                            }
                        }
                        (Op::Remove, Op::Add) => {
                            // Replace patch
                            if is_same_start {
                                // There can be no other pattern than these two.
                                // - e.g. Remove: 1657..1661(+3), Add: 1657..1661(+3) -> Replace: 1657..1661(+3)
                                // - e.g. Remove: 1657..1661(+3), Add: 1657..1662(+4) -> Replace: 1657..1662(+4)
                                *base_op = Op::Replace;
                                base_range.end = patch_range.end;
                                *base_value = patch_value;
                            } else {
                                // TODO: Discrete indices are treated as separate patches at a higher function stage before reaching this point.
                                // e.g. Remove: 1..5, Add: 10..11 -> Remove:1..5, Add: 10..11
                                // e.g. Remove: 1..5, Add: 4..11 -> Remove:1..3, Add: 4..11
                                // e.g. Remove: 10..12, Add: 4..10 -> Add: 4..10, Remove:11..12
                            }
                        }
                        (_base, _op) => {
                            #[cfg(feature = "tracing")]
                            tracing::warn!("Unsupported Descread pattern: {_base:?}, {_op:?} yet.")
                        }
                    }
                }
                OpRangeKind::Discrete(_op_ranges) => {}
                OpRangeKind::Pure(_) => {} // TODO: error occurred
            }
        }
        OpRangeKind::Discrete(_op_ranges) => {}
        OpRangeKind::Pure(op) => match op {
            Op::Add => {} // TODO: error occurred (unit value; e.g. 1, "string")
            Op::Replace => {
                base.value = patch_value;
            }
            Op::Remove => {
                remove(base);
            }
        },
    }
}

fn seq_add<'a>(
    base_range: &mut std::ops::Range<usize>,
    base_value: &mut Value<'a>,
    patch_range_len: usize,
    patch_value: Value<'a>,
) -> Result<(), MergeError> {
    let base_arr = base_value.try_as_array_mut()?;
    base_arr.par_extend(patch_value.try_into_array()?);
    base_range.end += patch_range_len;
    Ok(())
}

#[derive(Debug, snafu::Snafu)]
enum MergeError {
    #[snafu(transparent)]
    TryTypeError { source: simd_json::TryTypeError },
}

/// Remove & fallback to default
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
                value: json_typed!(borrowed, ["Animations\\1hm_UnsheatheAttack.hkx"]),
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
                value: json_typed!(borrowed, ["Animations\\MomoAJ\\mt_jumpfallback.hkx"]),
            },
        );

        merge_json_patches(&mut base_hash_map, add_hash_map);

        let mut expected = HashMap::new();
        let json_path = json_path!["#0009", "hkbProjectStringData", "characterFilenames",];
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

    #[test]
    fn test_seq_op_not_same_json_patches() {
        // Ex.1
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
                    range: 1656..1658,
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
                    range: 1656..1658,
                }),

                value: json_typed!(borrowed, ["Animations\\MomoAJ\\mt_jumpfallback.hkx"]),
            },
        );

        assert_eq!(base_hash_map, expected);
    }
}
