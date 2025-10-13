use crate::ptr_mut::PointerMut as _;
use crate::range::split_range::split_range_at_len;
use crate::vec_utils::{SmartExtend as _, SmartIntoIter as _, SmartIterMut as _};
use crate::{Action, JsonPatch, JsonPatchError, JsonPath, Op, Result, ValueWithPriority};
use core::ops::Range;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use simd_json::borrowed::Value;
use std::borrow::Cow;

const MARK_AS_REMOVED_STR: &str = "##Mark_As_Removed##"; // Separate inner str for test
const MARK_AS_REMOVED: Value<'static> = Value::String(Cow::Borrowed(MARK_AS_REMOVED_STR));

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
    #[cfg(feature = "tracing")]
    {
        let visualizer = visualize_ops(&patches)?;
        let target_len = target_array.len();
        tracing::debug!(
            "Seq merge conflict resolution:
Path: maybe asdsf, Seq target length: {target_len}
{visualizer}"
        );
    }

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
    let cmp_fn = |a: &ValueWithPriority<'a>, b: &ValueWithPriority<'a>| {
        let ValueWithPriority {
            patch: a,
            priority: a_priority,
        } = a;
        let ValueWithPriority {
            patch: b,
            priority: b_priority,
        } = b;

        let op_rank = |patch: &JsonPatch<'_>| match &patch.action {
            Action::Seq { op, .. } | Action::Pure { op } => match op {
                Op::Replace => 0,
                Op::Remove => 1,
                Op::Add => 2,
            },
            Action::SeqPush => 3,
        };

        a_priority.cmp(b_priority).then(op_rank(a).cmp(&op_rank(b)))
    };

    #[cfg(feature = "rayon")]
    patches.par_sort_unstable_by(cmp_fn);
    #[cfg(not(feature = "rayon"))]
    patches.sort_by(cmp_fn);
}

/// - [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=14cc7675b080133f94272b9ef3cc43ce)
///
/// # Assumptions
/// - patches are sorted.
fn apply_ops_parallel<'a>(
    mut base: Vec<Value<'a>>,
    patches: Vec<ValueWithPriority<'a>>,
) -> Result<Vec<Value<'a>>> {
    let (non_add_ops, mut add_ops): (Vec<_>, Vec<_>) =
        patches
            .smart_iter()
            .partition(|ValueWithPriority { patch, .. }| match &patch.action {
                Action::Pure { op } => matches!(op, Op::Replace | Op::Remove),
                Action::Seq { op, .. } => *op != Op::Add,
                Action::SeqPush => false,
            });

    // Apply Replace and Remove operations
    for ValueWithPriority { patch, priority } in non_add_ops {
        let JsonPatch { action: op, value } = patch;

        // Get sequence information if the action targets a sequence
        let (op, range) = op.try_as_seq()?;

        match op {
            Op::Replace => {
                let values = value_to_array(value)?;

                if let Some(add_patch) =
                    apply_replace_with_overflow(&mut base, range, values, priority)?
                {
                    add_ops.push(add_patch);
                }
            }
            Op::Remove => {
                let Some(slice) = base.get_mut(range.clone()) else {
                    return Err(JsonPatchError::UnexpectedRange {
                        patch_range: range,
                        actual_len: base.len(),
                    });
                };

                slice.smart_iter_mut().for_each(|element| {
                    *element = MARK_AS_REMOVED; // mark element for removal
                });
            }
            Op::Add => {} // Add should not appear here
        };
    }

    // Apply Add/SeqPush operations
    let mut offset = 0;
    for value in add_ops {
        match &value.patch.action {
            Action::Seq { op: Op::Add, range } => {
                let values = value_to_array(value.patch.value)?;
                let insert_at = range.start + offset;

                if insert_at < base.len() {
                    let values_len = values.len();
                    base.splice(insert_at..insert_at, values);
                    offset += values_len; // Update offset for subsequent inserts
                } else {
                    base.smart_extend(values);
                }
            }
            Action::SeqPush => {
                let values = value_to_array(value.patch.value)?;
                base.smart_extend(values); // Always append at the end
            }
            _ => {} // Should not appear here
        }
    }

    Ok(base)
}

/// Convert a `simd_json::Value` to a reference to an array (`Vec<Value>`).
///
/// # Why manual type checking?
/// Using `value.try_into_array()` will consume the value and on error the original
/// `Value` is not available, making it hard to include the actual value in error messages.
/// By manually matching the type, we can:
/// 1. Verify the type is correct.
/// 2. Return a reference to the array if successful.
/// 3. Include the original value in the error for better debugging/logging.
fn value_to_array<'a>(value: Value<'a>) -> Result<Vec<Value<'a>>, JsonPatchError> {
    match value {
        Value::Array(arr) => Ok(*arr),
        other => {
            let value_type = simd_json::base::TypedValue::value_type(&other);

            Err(JsonPatchError::try_type_from(
                simd_json::TryTypeError {
                    expected: simd_json::ValueType::Array,
                    got: value_type,
                },
                &["".into()],
                other.clone(),
            ))
        }
    }
}

type SplitValue<'a> = (
    Option<(Range<usize>, Vec<Value<'a>>)>,
    Option<(Range<usize>, Vec<Value<'a>>)>,
);

/// Splits a replacement operation into two parts:
/// - one that applies within bounds of `base`
/// - one that overflows and should be handled separately
///
/// # Returns
/// - `(in_bounds_range, in_bounds_values)`
/// - `(overflow_range, overflow_values)`
fn split_for_replace<'a>(
    range: Range<usize>,
    base_len: usize,
    values: Vec<Value<'a>>,
) -> SplitValue<'a> {
    let (in_range, overflow_range) = split_range_at_len(range, base_len);

    match (in_range, overflow_range) {
        (Some(in_r), Some(over_r)) => {
            let in_len = in_r.len();
            let (in_vals, overflow_vals) = values.split_at(in_len);
            (
                Some((in_r, in_vals.to_vec())),
                Some((over_r, overflow_vals.to_vec())),
            )
        }
        (Some(in_r), None) => (Some((in_r, values)), None),
        (None, Some(over_r)) => (None, Some((over_r, values))),
        (None, None) => (None, None), // Should never happen
    }
}

fn apply_replace_with_overflow<'a>(
    base: &mut Vec<Value<'a>>,
    range: Range<usize>,
    values: Vec<Value<'a>>,
    priority: usize,
) -> Result<Option<ValueWithPriority<'a>>> {
    let (in_bounds_opt, overflow_opt) = split_for_replace(range.clone(), base.len(), values);

    if let Some((in_bounds_range, in_bounds_values)) = in_bounds_opt {
        let Some(slice) = base.get_mut(in_bounds_range) else {
            return Err(JsonPatchError::UnexpectedRange {
                patch_range: range,
                actual_len: base.len(),
            });
        };

        slice
            .smart_iter_mut()
            .zip(in_bounds_values)
            .for_each(|(element, patch)| {
                *element = patch;
            });
    }

    if let Some((_overflow_range, overflow_values)) = overflow_opt {
        #[cfg(feature = "tracing")]
        tracing::info!(
            "Replace overflow: attempted to write to range {range:?} (base.len() = {}); \
                overflowed into range {_overflow_range:?} with {} remaining values",
            base.len(),
            overflow_values.len()
        );

        if !overflow_values.is_empty() {
            return Ok(Some(ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op: Op::Add,
                        range: base.len()..base.len(),
                    },
                    value: overflow_values.into(),
                },
                priority,
            }));
        }
    }

    Ok(None)
}

#[cfg(any(feature = "tracing", test))]
#[allow(clippy::cognitive_complexity)]
fn visualize_ops(patches: &[ValueWithPriority<'_>]) -> Result<String, JsonPatchError> {
    use simd_json::derived::ValueTryAsArray as _;
    use std::collections::BTreeSet;

    #[derive(Debug, Clone, Copy)]
    enum DisplayIndex {
        Index(usize),
        Ellipsis,
    }

    const CELL_WIDTH: usize = 5;
    const SPACE_SYMBOL: &str = "     ";
    const ADD_SYMBOL: &str = " [+] ";
    const REPLACE_SYMBOL: &str = " [*] ";
    const REMOVE_SYMBOL: &str = " [-] ";
    const PUSH_SYMBOL: &str = " [>] ";
    const ELLIPSIS: &str = " ... ";
    const GAP_THRESHOLD: usize = 5;

    // --- 1. collect used indices
    let mut indices = BTreeSet::new();
    let mut max_index = 0;

    for p in patches {
        match &p.patch.action {
            Action::Seq { range, .. } => {
                for i in range.clone() {
                    indices.insert(i);
                }
                max_index = max_index.max(range.end);
            }
            Action::SeqPush => {
                let push_len = p.patch.value.try_as_array().map(|a| a.len()).unwrap_or(1);
                for i in max_index..(max_index + push_len) {
                    indices.insert(i);
                }
                max_index += push_len;
            }
            Action::Pure { .. } => {}
        }
    }

    if indices.is_empty() {
        return Ok(String::new());
    }

    // --- 2. compress indices with ellipsis markers
    let mut display_indices = Vec::new();
    let all_indices: Vec<_> = indices.iter().copied().collect();

    let mut last = all_indices[0];
    for &v in &all_indices {
        if v > last + GAP_THRESHOLD {
            display_indices.push(DisplayIndex::Ellipsis);
        }
        display_indices.push(DisplayIndex::Index(v));
        last = v;
    }

    let sep_line = {
        let mut sep_line = "--------|-----|--".to_string();
        sep_line.push_str(&"-".repeat(display_indices.len() * CELL_WIDTH));
        sep_line.push('\n');
        sep_line
    };

    // --- 3. build header
    let mut out = String::new();
    out.push_str("Op      | Ord |");
    for idx in &display_indices {
        match idx {
            DisplayIndex::Index(i) => out.push_str(&format!("{i:^CELL_WIDTH$}")),
            DisplayIndex::Ellipsis => out.push_str(&format!("{ELLIPSIS:^CELL_WIDTH$}")),
        }
    }
    out.push_str(" |\n");
    out.push_str(&sep_line);

    // --- 4. sort operations
    let mut sorted = patches.to_vec();
    sorted.sort_by(|a, b| {
        use Action::*;
        let rank = |act: &Action| match act {
            Seq { op, .. } => match op {
                Op::Replace | Op::Remove => 0,
                Op::Add => 1,
            },
            SeqPush => 2,
            Pure { .. } => 3,
        };
        let a_rank = rank(&a.patch.action);
        let b_rank = rank(&b.patch.action);
        a_rank.cmp(&b_rank).then(a.priority.cmp(&b.priority))
    });

    // --- 5. render each op row
    for patch in &sorted {
        let (label, draw): (&str, Box<dyn Fn(usize) -> &'static str>) = match &patch.patch.action {
            Action::Seq { op, range } => match op {
                Op::Add => (
                    "Add",
                    Box::new(move |i| {
                        if i >= range.start && i < range.end {
                            ADD_SYMBOL
                        } else {
                            SPACE_SYMBOL
                        }
                    }),
                ),
                Op::Replace => (
                    "Replace",
                    Box::new(move |i| {
                        if i >= range.start && i < range.end {
                            REPLACE_SYMBOL
                        } else {
                            SPACE_SYMBOL
                        }
                    }),
                ),
                Op::Remove => (
                    "Remove",
                    Box::new(move |i| {
                        if i >= range.start && i < range.end {
                            REMOVE_SYMBOL
                        } else {
                            SPACE_SYMBOL
                        }
                    }),
                ),
            },
            Action::SeqPush => {
                let push_len = patch
                    .patch
                    .value
                    .try_as_array()
                    .map(|a| a.len())
                    .unwrap_or(1);
                let start = max_index - push_len;
                (
                    "Push",
                    Box::new(move |i| {
                        if i >= start && i < start + push_len {
                            PUSH_SYMBOL
                        } else {
                            SPACE_SYMBOL
                        }
                    }),
                )
            }
            Action::Pure { .. } => continue,
        };

        out.push_str(&format!("{:<7} | {:>3} |", label, patch.priority));
        for idx in &display_indices {
            match idx {
                DisplayIndex::Index(i) => out.push_str(draw(*i)),
                DisplayIndex::Ellipsis => out.push_str(ELLIPSIS),
            }
        }
        out.push_str(" |\n");
    }

    // --- 6. render result line with original indices for removed elements
    {
        let mut orig_indices: Vec<Option<usize>> = (0..max_index).map(Some).collect();

        // --- simulate the effect of patches on original indices
        for patch in &sorted {
            match &patch.patch.action {
                Action::Seq { op, range } => match op {
                    Op::Add => {
                        // Insert None for new elements (shifts right the existing indices)
                        let insert_at = range.start.min(orig_indices.len());
                        for _ in 0..(range.end - range.start) {
                            orig_indices.insert(insert_at, None);
                        }
                    }
                    Op::Remove | Op::Replace => {
                        // Replace does not shift indices
                        // Remove elements (left shift the rest)
                        // No action needed here; orig_indices already contains Some(index)
                    }
                },
                Action::SeqPush => {
                    let push_len = patch
                        .patch
                        .value
                        .try_as_array()
                        .map(|a| a.len())
                        .unwrap_or(1);
                    for _ in 0..push_len {
                        orig_indices.push(None);
                    }
                }
                Action::Pure { .. } => {}
            }
        }

        // Build Result line
        out.push_str(&sep_line);
        out.push_str("Result  |     |");
        for cell in display_indices.iter() {
            match cell {
                DisplayIndex::Index(idx) => {
                    // Check if this index was a Remove original
                    let mut sym = if let Some(orig) = orig_indices.get(*idx).copied().flatten() {
                        format!(" [{orig}] ").into()
                    } else {
                        Cow::Borrowed(SPACE_SYMBOL)
                    };

                    // Override with Add / Replace / Push symbols if applicable
                    for patch in &sorted {
                        match &patch.patch.action {
                            Action::Seq { op, range } => {
                                if *idx >= range.start && *idx < range.end {
                                    sym = match op {
                                        Op::Add => ADD_SYMBOL.into(),
                                        Op::Replace => REPLACE_SYMBOL.into(),
                                        Op::Remove => sym, // keep original index
                                    };
                                }
                            }
                            Action::SeqPush => {
                                let push_len = patch
                                    .patch
                                    .value
                                    .try_as_array()
                                    .map(|a| a.len())
                                    .unwrap_or(1);
                                let start = max_index - push_len;
                                if *idx >= start && *idx < start + push_len {
                                    sym = PUSH_SYMBOL.into();
                                }
                            }
                            Action::Pure { .. } => {}
                        }
                    }

                    out.push_str(&sym);
                }
                DisplayIndex::Ellipsis => out.push_str(ELLIPSIS),
            }
        }
        out.push_str(" |\n");
        out.push_str(&sep_line);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seq_patch_with_push() {
        let mut patches: Vec<ValueWithPriority<'_>> = vec![
            // Add in the middle
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op: Op::Add,
                        range: 1..3,
                    },
                    value: simd_json::json_typed! {borrowed, ["a", "b"]},
                },
                priority: 1,
            },
            // Replace a range
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op: Op::Replace,
                        range: 4..6,
                    },
                    value: simd_json::json_typed! {borrowed, ["x1", "x2"]},
                },
                priority: 0,
            },
            // Remove a range
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op: Op::Remove,
                        range: 2..4,
                    },
                    value: simd_json::json_typed! {borrowed, []},
                },
                priority: 2,
            },
            // SeqPush: append to the end
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed! {borrowed, ["P1", "P2"]},
                },
                priority: 1,
            },
        ];

        // Visualizer before applying patches
        let visual = visualize_ops(&patches).unwrap();

        // Apply patches
        sort_by_priority(&mut patches);
        let mut base_seq: Vec<Value<'_>> = (0..6).map(|i| i.to_string().into()).collect();
        apply_seq_array_directly(&mut base_seq, patches).unwrap();

        // Expected array after patches
        let expected: Vec<Value> = vec!["0", "a", "b", "1", "x1", "x2", "P1", "P2"]
            .smart_iter()
            .map(|v| v.into())
            .collect();

        assert_eq!(
            base_seq, expected,
            "Final patched array does not match expected"
        );

        println!("{visual}");

        let expected_visual = "\
Op      | Ord |  1    2    3    4    5    6    7   |\n\
--------|-----|-------------------------------------\n\
Replace |   0 |                [*]  [*]            |\n\
Remove  |   2 |      [-]  [-]                      |\n\
Add     |   1 | [+]  [+]                           |\n\
Push    |   1 |                          [>]  [>]  |\n\
--------|-----|-------------------------------------\n\
Result  |     | [+]  [+]  [1]  [*]  [*]  [>]  [>]  |\n\
--------|-----|-------------------------------------\n\
";

        assert_eq!(
            visual, expected_visual,
            "Visualizer output does not match expected"
        );
    }
}
