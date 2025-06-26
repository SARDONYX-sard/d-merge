use crate::ptr_mut::PointerMut as _;
use crate::vec_utils::{SmartExtend as _, SmartIntoIter as _, SmartIterMut as _};
use crate::{JsonPatch, JsonPatchError, JsonPath, Op, Result, ValueWithPriority};
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use simd_json::{borrowed::Value, derived::ValueTryIntoArray};
use std::borrow::Cow;

const MARK_AS_REMOVED: Value<'static> = Value::String(Cow::Borrowed("##Mark_As_Removed##"));

/// Replace one value.
///
/// # Note
/// - Support `Object` or `Array`
/// - Unsupported range remove. use `apply_range` instead
///
/// # Errors
/// Failed to apply
pub fn apply_seq_by_priority<'a>(
    file_name: &str,
    json: &mut Value<'a>,
    path: JsonPath<'a>,
    mut patches: Vec<ValueWithPriority<'a>>,
) -> Result<()> {
    let _ = file_name;
    let target = json
        .ptr_mut(&path)
        .ok_or_else(|| JsonPatchError::not_found_target_from(&path, &patches))?;

    let Value::Array(template_array) = target else {
        return Err(JsonPatchError::unsupported_range_kind_from(&path, &patches));
    };

    sort_by_priority(patches.as_mut_slice());
    #[cfg(feature = "tracing")]
    {
        let path = path.join("/");
        let target_len = template_array.len();
        let visualizer = visualize_ops(&patches)?;
        tracing::debug!(
            "Seq merge conflict resolution for `{file_name}` file:
Path: {path}, Seq target length: {target_len}
{visualizer}"
        );
    }

    let patch_target_vec = core::mem::take(template_array);
    let patched_array = apply_ops_parallel(*patch_target_vec, patches)?
        .smart_iter()
        .filter(|v| v != &MARK_AS_REMOVED);

    template_array.smart_extend(patched_array);

    Ok(())
}

/// Resolve conflicts in order of priority and apply them to the array.
///
/// This function is applied directly to the target array without specifying a JSON path.
///
/// # Errors
/// Returns an error if applying the patches fails.
pub fn apply_seq_array_directly<'a>(
    target_array: &mut Vec<Value<'a>>,
    mut patches: Vec<ValueWithPriority<'a>>,
) -> Result<()> {
    let patch_target_vec = core::mem::take(target_array);
    sort_by_priority(patches.as_mut_slice());
    let patched_array = apply_ops_parallel(patch_target_vec, patches)?
        .smart_iter()
        .filter(|v| v != &MARK_AS_REMOVED);
    target_array.smart_extend(patched_array);
    Ok(())
}

// Separate sorted ops into Add and others
fn sort_by_priority<'a>(patches: &mut [ValueWithPriority<'a>]) {
    #[cfg(feature = "rayon")]
    patches.par_sort_unstable_by(|a, b| {
        let ValueWithPriority {
            patch: a,
            priority: a_priority,
        } = a;
        let ValueWithPriority {
            patch: b,
            priority: b_priority,
        } = b;

        let op_rank =
            |patch: &JsonPatch<'_>| match patch.op.try_as_seq().map(|op| op.op).unwrap_or_default()
            {
                Op::Replace => 0,
                Op::Remove => 1,
                Op::Add => 2,
            };

        a_priority.cmp(b_priority).then(op_rank(a).cmp(&op_rank(b)))
    });

    #[cfg(not(feature = "rayon"))]
    patches.sort_by(|a, b| {
        let ValueWithPriority {
            patch: a,
            priority: a_priority,
        } = a;
        let ValueWithPriority {
            patch: b,
            priority: b_priority,
        } = b;

        let op_rank =
            |patch: &JsonPatch<'_>| match patch.op.try_as_seq().map(|op| op.op).unwrap_or_default()
            {
                Op::Replace => 0,
                Op::Remove => 1,
                Op::Add => 2,
            };

        a_priority.cmp(b_priority).then(op_rank(a).cmp(&op_rank(b)))
    });
}

/// - [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=14cc7675b080133f94272b9ef3cc43ce)
///
/// # Assumptions
/// - patches are sorted.
fn apply_ops_parallel<'a>(
    mut base: Vec<Value<'a>>,
    patches: Vec<ValueWithPriority<'a>>,
) -> Result<Vec<Value<'a>>> {
    let (non_add_ops, add_ops): (Vec<_>, Vec<_>) =
        patches
            .into_par_iter()
            .partition(|ValueWithPriority { patch, .. }| {
                patch.op.try_as_seq().map(|op| op.op).unwrap_or_default() != Op::Add
            });

    // Replace, Remove
    for ValueWithPriority { patch, .. } in non_add_ops {
        let JsonPatch { op, value } = patch;
        let seq = op.try_as_seq()?;
        match seq.op {
            Op::Replace => {
                let values = value
                    .try_into_array()
                    .map_err(|err| JsonPatchError::try_type_from(err, &["".into()], ""))?;
                let range = seq.range.clone();
                let Some(slice) = base.get_mut(range.clone()) else {
                    return Err(JsonPatchError::UnexpectedRange {
                        patch_range: range,
                        actual_len: base.len(),
                    });
                };
                slice
                    .smart_iter_mut()
                    .zip(values)
                    .for_each(|(element, patch)| {
                        *element = patch;
                    });
            }
            Op::Remove => {
                let range = seq.range.clone();
                let Some(slice) = base.get_mut(range.clone()) else {
                    return Err(JsonPatchError::UnexpectedRange {
                        patch_range: range,
                        actual_len: base.len(),
                    });
                };
                slice.smart_iter_mut().for_each(|element| {
                    *element = MARK_AS_REMOVED;
                });
            }
            Op::Add => {}
        };
    }

    // Add
    let mut offset = 0;
    for value in add_ops {
        let seq = value.patch.op.try_as_seq()?;
        let values = value
            .patch
            .value
            .try_into_array()
            .map_err(|err| JsonPatchError::try_type_from(err, &["".into()], ""))?;
        let insert_at = seq.range.start + offset;

        if insert_at <= base.len() {
            let values_len = values.len();
            base.splice(insert_at..insert_at, values);
            offset += values_len;
        } else {
            base.smart_extend(values);
        }
    }

    Ok(base)
}

#[cfg(any(feature = "tracing", test))]
fn visualize_ops(patches: &[ValueWithPriority<'_>]) -> Result<String, JsonPatchError> {
    use std::collections::BTreeSet;

    const CELL_WIDTH: usize = 5;
    const SPACE_SYMBOL: &str = "     ";
    const ADD_SYMBOL: &str = " [+] ";
    const REPLACE_SYMBOL: &str = " [*] ";
    const REMOVE_SYMBOL: &str = " [-] ";
    const MAX_INLINE_GAP: usize = 2;

    const _: () = {
        assert!(CELL_WIDTH == SPACE_SYMBOL.len());
        assert!(CELL_WIDTH == ADD_SYMBOL.len());
        assert!(CELL_WIDTH == REPLACE_SYMBOL.len());
        assert!(CELL_WIDTH == REMOVE_SYMBOL.len());
    };

    // 1. collect all used indexes
    let mut indices = BTreeSet::new();
    let mut max_index = 0;

    // 1. collect all used indices (0-based)
    for patch in patches {
        let seq = patch.patch.op.try_as_seq()?;
        max_index = max_index.max(seq.range.end);
        for i in seq.range.start..seq.range.end {
            indices.insert(i);
        }
    }

    // 2. build display index list with ellipsis
    let mut display_indices = Vec::new();
    let mut last = 0;
    for &i in &indices {
        if i > last + MAX_INLINE_GAP + 1 {
            display_indices.push(0); // use 0 as marker for ellipsis
        }
        display_indices.push(i);
        last = i;
    }

    // 3. render index row
    let mut output = String::new();
    for &i in &display_indices {
        if i == 0 {
            output.push_str(" ... ");
        } else {
            output.push_str(&format!("{i:^width$}", width = CELL_WIDTH));
        }
    }
    output.push_str("| Op      | priority |\n");

    // 4. render each patch line
    for patch in patches {
        let seq = patch.patch.op.try_as_seq()?;
        let range = seq.range.clone();
        let op = seq.op;

        for &i in &display_indices {
            let symbol = if i == 0 {
                " ... "
            } else if i >= range.start && i < range.end {
                match op {
                    Op::Add => ADD_SYMBOL,
                    Op::Replace => REPLACE_SYMBOL,
                    Op::Remove => REMOVE_SYMBOL,
                }
            } else {
                SPACE_SYMBOL
            };
            output.push_str(symbol);
        }

        let label = format!(
            "| {:<7} | {:>8} |",
            match op {
                Op::Add => "Add",
                Op::Remove => "Remove",
                Op::Replace => "Replace",
            },
            patch.priority
        );
        output.push_str(&label);
        output.push('\n');
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpRange, OpRangeKind};

    #[test]
    fn test_seq_patch() {
        let mut patches: Vec<ValueWithPriority<'_>> = vec![
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 1..5,
                    }),
                    value: simd_json::json_typed! {borrowed, [ "a", "b", "c", "d" ]},
                },
                priority: 1,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Replace,
                        range: 10..13,
                    }),
                    value: simd_json::json_typed! {borrowed, ["x1", "x2", "x3"]},
                },
                priority: 0,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Remove,
                        range: 3..7,
                    }),
                    value: simd_json::json_typed! {borrowed, []},
                },
                priority: 2,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 18..21,
                    }),
                    value: simd_json::json_typed! {borrowed, ["y1", "y2", "y3"]},
                },
                priority: 1,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Remove,
                        range: 2..9,
                    }),
                    value: simd_json::json_typed! {borrowed, []},
                },
                priority: 2,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Seq(OpRange {
                        op: Op::Add,
                        range: 21..23,
                    }),
                    value: simd_json::json_typed! {borrowed, ["over1", "over2", "over3"]},
                },
                priority: 1,
            },
        ];

        let base_seq: Vec<String> = (0..21).map(|i| i.to_string()).collect();
        let mut base_seq: Vec<Value<'_>> = base_seq.smart_iter().map(|i| i.into()).collect();

        println!("Operation Map:\n{}", visualize_ops(&patches).unwrap());
        sort_by_priority(&mut patches);
        apply_seq_array_directly(&mut base_seq, patches).unwrap();
        println!("{base_seq:#?}");
    }
}
