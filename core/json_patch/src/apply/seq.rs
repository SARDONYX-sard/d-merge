use crate::ptr_mut::PointerMut as _;
use crate::range::split_range::split_range_at_len;
use crate::vec_utils::{SmartExtend as _, SmartIntoIter as _, SmartIterMut as _};
use crate::{Action, JsonPatch, JsonPatchError, JsonPath, Op, Result, ValueWithPriority};
use core::ops::Range;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use simd_json::borrowed::Value;
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

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
        let visualizer = visualize_ops(&patches, target_len);
        tracing::debug!(
            "Seq Json Patch Conflict Resolution report
 file=\"{file_name}\"
 path: {path}(len: {target_len})
---
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
/// This function applies multiple sequence-type JSON patches directly
/// to a mutable JSON array, resolving conflicts by patch priority.
///
/// # Behavior Notes
/// - Patches are applied in ascending order of `priority`.
/// - `Replace` with fewer elements than its range implicitly removes the extra elements.
/// - This function directly modifies the array in place.
///
/// # Errors
/// Returns [`JsonPatchError`] if the patch fails or if the target is not an array.
///
/// # Example
/// ```
/// use simd_json::{base::ValueTryAsArrayMut as _, borrowed::Value,json_typed};
/// use json_patch::{apply_seq_array_directly, JsonPatch, Action, Op, ValueWithPriority, JsonPatchError};
///
/// fn main() -> Result<(), JsonPatchError> {
///     // Prepare a mix of sequence operations with different priorities.
///     let patches: Vec<ValueWithPriority<'_>> = vec![
///         // 1/4 Replace elements 1..4 with ["A", "B"].
///         // Range is longer than replacement (3 vs 2), so 1 element will be removed.
///         ValueWithPriority {
///             patch: JsonPatch {
///                 action: Action::Seq {
///                     op: Op::Replace,
///                     range: 1..4,
///                 },
///                 value: json_typed! {borrowed, ["A", "B"]},
///             },
///             priority: 0,
///         },
///
///         // 2/4 Add ["X"] before index 2 (after Replace has adjusted positions).
///         ValueWithPriority {
///             patch: JsonPatch {
///                 action: Action::Seq {
///                     op: Op::Add,
///                     range: 2..2,
///                 },
///                 value: json_typed! {borrowed, ["X"]},
///             },
///             priority: 1,
///         },
///
///         // 3/4 Remove elements 0..1 (remove the first element).
///         ValueWithPriority {
///             patch: JsonPatch {
///                 action: Action::Seq {
///                     op: Op::Remove,
///                     range: 0..1,
///                 },
///                 value: json_typed! {borrowed, []},
///             },
///             priority: 2,
///         },
///
///         // 4/4 Append ["Z1", "Z2"] to the end.
///         ValueWithPriority {
///             patch: JsonPatch {
///                 action: Action::SeqPush,
///                 value: json_typed! {borrowed, ["Z1", "Z2"]},
///             },
///             priority: 1,
///         },
///     ];
///
///     // Initial array
///     let array_path = json_patch::json_path!["Example", "array"];
///     let mut actual = json_typed!(borrowed, ["0", "1", "2", "3", "4", "5"]);
///
///     // Get a mutable reference to the array value.
///     let seq = match actual.try_as_array_mut() {
///         Ok(seq) => seq,
///         Err(e) => return Err(JsonPatchError::try_type_from(e, &array_path, &actual)),
///     };
///
///     // Apply all patches directly.
///     apply_seq_array_directly(seq, patches)?;
///
///     // Step-by-step visualization:
///     // 1. Replace(1..4, ["A","B"]) → ["0", "A", "B", "4", "5"]
///     // 2️. Remove mark(0..1)        → ["⛔", "A", "B", "4", "5"]
///     // 3️. Add(2..2, ["X"])         → ["⛔", "A", "X", "B", "4", "5"]
///     // 4️. SeqPush(["Z1","Z2"])     → ["⛔", "A", "X", "B", "4", "5", "Z1", "Z2"]
///     // 5️. Final remove pass        → ["A", "X", "B", "4", "5", "Z1", "Z2"]
///
///     let expected = json_typed!(borrowed, ["A", "X", "B", "4", "5", "Z1", "Z2"]);
///     assert_eq!(actual, expected);
///
///     Ok(())
/// }
/// ```
pub fn apply_seq_array_directly<'a>(
    target_array: &mut Vec<Value<'a>>,
    mut patches: Vec<ValueWithPriority<'a>>,
) -> Result<()> {
    #[cfg(feature = "tracing")]
    {
        let visualizer = visualize_ops(&patches, target_array.len());
        let target_len = target_array.len();
        tracing::debug!(
            "Seq Json Patch Conflict Resolution\n target_len={target_len}\n ---\n{visualizer}"
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
                Action::Seq { op, .. } => matches!(op, Op::Replace | Op::Remove),
                Action::SeqPush => false,
            });

    // Apply Replace and Remove operations
    for ValueWithPriority { patch, priority } in non_add_ops {
        let JsonPatch { action, value } = patch;

        // Get sequence information if the action targets a sequence
        let (op, range) = action.try_as_seq()?;

        match op {
            Op::Replace => {
                let values = value_as_array(value)?;

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
                let values = value_as_array(value.patch.value)?;
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
                let values = value_as_array(value.patch.value)?;
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
fn value_as_array<'a>(value: Value<'a>) -> Result<Vec<Value<'a>>, JsonPatchError> {
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
                other,
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
    mut values: Vec<Value<'a>>,
) -> SplitValue<'a> {
    let (in_range, overflow_range) = split_range_at_len(range, base_len);

    match (in_range, overflow_range) {
        (Some(in_r), Some(over_r)) => {
            let in_len = in_r.len();
            let overflow_vals = values.split_off(in_len);
            (Some((in_r, values)), Some((over_r, overflow_vals)))
        }
        (Some(in_r), None) => (Some((in_r, values)), None),
        (None, Some(over_r)) => (None, Some((over_r, values))),
        (None, None) => (None, None), // unreadable
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
        #[cfg(feature = "tracing")]
        let cloned_in_bounds_range = in_bounds_range.clone();

        let Some(slice) = base.get_mut(in_bounds_range) else {
            return Err(JsonPatchError::UnexpectedRange {
                patch_range: range,
                actual_len: base.len(),
            });
        };

        let written = AtomicUsize::new(0);
        slice
            .smart_iter_mut()
            .zip(in_bounds_values)
            .for_each(|(element, patch)| {
                *element = patch;
                written.fetch_add(1, Ordering::Relaxed);
            });

        let written_count = written.load(Ordering::Relaxed);
        if written_count < slice.len() {
            let remain_range = written_count..slice.len();
            #[cfg(feature = "tracing")]
            tracing::info!(
                "[Seq: Replace as Remove] Replace range {cloned_in_bounds_range:?}: only {written_count} values provided for {} elements, \
                marking remaining {remain_range:?} elements as removed",
                slice.len(),
            );
            slice[remain_range]
                .smart_iter_mut()
                .for_each(|element| *element = MARK_AS_REMOVED);
        }
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
fn visualize_ops(patches: &[ValueWithPriority<'_>], target_array_len: usize) -> String {
    const SPACE_SYMBOL: &str = "     ";
    const ADD_SYMBOL: &str = " [+] ";
    const REPLACE_SYMBOL: &str = " [*] ";
    const REMOVE_SYMBOL: &str = " [-] ";
    const PUSH_SYMBOL: &str = " [>] ";
    const ELLIPSIS: &str = " ... ";
    const GAP_THRESHOLD: usize = 20;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum ActionType {
        Replace,
        Remove,
        Add,
        Push,
    }

    impl ActionType {
        const fn symbol(&self) -> &'static str {
            match self {
                Self::Replace => REPLACE_SYMBOL,
                Self::Remove => REMOVE_SYMBOL,
                Self::Add => ADD_SYMBOL,
                Self::Push => PUSH_SYMBOL,
            }
        }

        const fn as_str(&self) -> &'static str {
            match self {
                Self::Replace => "Replace",
                Self::Remove => "Remove",
                Self::Add => "Add",
                Self::Push => "Push",
            }
        }

        const fn rank(&self) -> usize {
            match self {
                Self::Replace => 0,
                Self::Remove => 1,
                Self::Add => 2,
                Self::Push => 3,
            }
        }
    }

    #[derive(Debug)]
    struct TableRow {
        op: ActionType,
        priority: usize,
        range: Range<usize>,
    }

    // --- 1. convert patches to TableRow
    let max_index = AtomicUsize::new(0);
    let mut rows: Vec<TableRow> = patches
        .smart_iter()
        .filter_map(|patch| {
            match &patch.patch.action {
                Action::Seq { op, range } => {
                    let action_type = match op {
                        Op::Add => ActionType::Add,
                        Op::Replace => ActionType::Replace,
                        Op::Remove => ActionType::Remove,
                    };
                    max_index.fetch_max(range.end, Ordering::Relaxed);
                    Some(TableRow {
                        op: action_type,
                        priority: patch.priority,
                        range: range.clone(),
                    })
                }
                Action::SeqPush => {
                    let push_len =
                        simd_json::derived::ValueTryAsArray::try_as_array(&patch.patch.value)
                            .map(|a| a.len())
                            .unwrap_or(1);

                    let start = target_array_len;
                    let end = start + push_len;
                    max_index.fetch_max(end, Ordering::Relaxed);
                    Some(TableRow {
                        op: ActionType::Push,
                        priority: patch.priority,
                        range: start..end,
                    })
                }
                Action::Pure { .. } => None, // skip
            }
        })
        .collect();
    let max_index = max_index.load(Ordering::Relaxed);

    if rows.is_empty() {
        return String::new();
    }
    let cell_width = match max_index {
        0..=9 => 5,   // <- e.g. ` 0-9 `.len()
        10..=99 => 7, // <- e.g. ` 98-99 `.len()
        100..=999 => 9,
        1000..=9999 => 11,
        _ => 13, // safety, though impossible
    };

    // --- 2. collect all start/end points for non-overlapping segments
    let mut points = std::collections::BTreeSet::new();
    for row in &rows {
        points.insert(row.range.start);
        points.insert(row.range.end);
    }
    let points: Vec<_> = points.smart_iter().collect();

    // --- 3. create segments
    let mut segments = Vec::new();
    for w in points.windows(2) {
        segments.push(w[0]..w[1]);
    }

    // --- 4. build header
    let mut header = String::new();
    header.push_str("Op      | Ord |");
    let mut last = None;

    for seg in &segments {
        if let Some(prev) = last {
            if seg.start > prev + GAP_THRESHOLD {
                header.push_str(&format!("{ELLIPSIS:^cell_width$}"));
            }
        }

        if seg.end == seg.start + 1 {
            // single index
            header.push_str(&format!("{:^cell_width$}", seg.start));
        } else {
            // multiple indices
            header.push_str(&format!(
                "{:^cell_width$}",
                format!("{}-{}", seg.start, seg.end - 1)
            ));
        }

        last = Some(seg.end - 1);
    }
    header.push_str("|\n");

    // --- 5. separator line
    let sep_line = {
        let mut sep_line = "-".repeat(header.len() - 1);
        sep_line.push('\n');
        sep_line
    };

    let mut out = String::new();
    out.push_str(&header);
    out.push_str(&sep_line);

    // --- 6. sort rows by op rank then priority
    {
        let priority_sort = |a: &TableRow, b: &TableRow| {
            a.op.rank()
                .cmp(&b.op.rank())
                .then(a.priority.cmp(&b.priority))
        };
        #[cfg(feature = "rayon")]
        rows.par_sort_unstable_by(priority_sort);
        #[cfg(not(feature = "rayon"))]
        rows.sort_by(priority_sort);
    }

    // --- 7. render each row
    for row in &rows {
        out.push_str(&format!("{:<7} | {:>3} |", row.op.as_str(), row.priority));
        let mut last = None;
        for seg in &segments {
            if let Some(prev) = last {
                if seg.start > prev + GAP_THRESHOLD {
                    out.push_str(ELLIPSIS);
                }
            }
            last = Some(seg.end - 1);

            // check if this segment overlaps with row.range
            let content = if row.range.start < seg.end && row.range.end > seg.start {
                row.op.symbol()
            } else {
                SPACE_SYMBOL
            };
            out.push_str(&format!("{content:^cell_width$}"));
        }
        out.push_str("|\n");
    }

    // --- 8. final separator
    out.push_str(&sep_line);

    out
}

#[cfg(test)]
mod tests {
    use simd_json::{base::ValueTryAsArrayMut as _, json_typed};

    use super::*;

    #[test]
    fn test_replace_with_less_values_than_range() {
        // replace range 1..4 but has 2
        let patches: Vec<ValueWithPriority<'_>> = vec![ValueWithPriority {
            patch: JsonPatch {
                action: Action::Seq {
                    op: Op::Replace,
                    range: 1..4,
                },
                value: json_typed! {borrowed, ["A", "B"]},
            },
            priority: 0,
        }];

        let mut actual = json_typed!(borrowed, ["0", "1", "2", "3", "4", "5"]);
        apply_seq_array_directly(actual.try_as_array_mut().unwrap(), patches).unwrap();

        let expected = json_typed!(borrowed, ["0", "A", "B", "4", "5"]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_seq_patch_with_push() {
        let patches: Vec<ValueWithPriority<'_>> = vec![
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

        let mut actual = json_typed!(borrowed, ["0", "1", "2", "3", "4", "5"]);
        let seq_mut = actual.try_as_array_mut().unwrap();
        let visual = visualize_ops(&patches, seq_mut.len());
        apply_seq_array_directly(seq_mut, patches).unwrap();

        let expected = json_typed!(borrowed, ["0", "a", "b", "1", "x1", "x2", "P1", "P2"]);
        assert_eq!(actual, expected);

        const EXPECTED_VISUAL: &str = "\
Op      | Ord |  1    2    3   4-5  6-7 |\n\
-----------------------------------------\n\
Replace |   0 |                [*]      |\n\
Remove  |   2 |      [-]  [-]           |\n\
Add     |   1 | [+]  [+]                |\n\
Push    |   1 |                     [>] |\n\
-----------------------------------------\n\
";
        println!("{visual}");
        assert_eq!(visual, EXPECTED_VISUAL);
    }
}
