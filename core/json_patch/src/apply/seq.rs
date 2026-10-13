#[cfg(any(feature = "tracing", test))]
use core::ops::Range;

use simd_json::borrowed::Value;

use crate::{
    Action, JsonPatch, JsonPatchError, JsonPath, Op, Result, ValueWithPriority,
    ptr_mut::PointerMut as _, range::split_range::split_range_at_len,
};

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
    // NOTE: The visualizer is expensive, so build it only when it is actually emitted.
    #[cfg(feature = "tracing")]
    if tracing::enabled!(tracing::Level::TRACE) {
        let path = path.join("/");
        let target_len = template_array.len();
        let visualizer = visualize_ops(&patches, target_len);
        tracing::trace!(
            "Seq Json Patch Conflict Resolution report
 file=\"{file_name}\"
 path: {path}(len: {target_len})
---
{visualizer}"
        );
    }

    let patch_target_vec = core::mem::take(&mut **template_array);
    **template_array = apply_ops(patch_target_vec, patches)?;

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
    if tracing::enabled!(tracing::Level::TRACE) {
        let visualizer = visualize_ops(&patches, target_array.len());
        let target_len = target_array.len();
        tracing::trace!(
            "Seq Json Patch Conflict Resolution\n target_len={target_len}\n ---\n{visualizer}"
        );
    }

    let patch_target_vec = core::mem::take(target_array);
    sort_by_priority(patches.as_mut_slice());
    *target_array = apply_ops(patch_target_vec, patches)?;
    Ok(())
}

// Separate sorted ops into Add and others
fn sort_by_priority<'a>(patches: &mut [ValueWithPriority<'a>]) {
    let cmp_fn = |a: &ValueWithPriority<'a>, b: &ValueWithPriority<'a>| {
        let ValueWithPriority { patch: a, priority: a_priority } = a;
        let ValueWithPriority { patch: b, priority: b_priority } = b;

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

    // NOTE: Must be a stable sort. Patches with the same `(priority, op)` keep their input order,
    //       which makes the result deterministic.
    patches.sort_by(cmp_fn);
}

/// Applies sorted sequence patches to `base` in a single pass.
///
/// All positions refer to the indices of the **original** `base` array.
///
/// # Algorithm
/// 1. `Replace`/`Remove` are applied in place; removal is tracked by a flag per element
///    (the length of `base` does not change during this step).
/// 2. `Add`/`SeqPush` are collected as `(insert position, values)` and stably sorted by position.
///    Same-position inserts therefore keep the priority order.
/// 3. The result is built in one pass: inserts, then kept elements, then the overflow of `Replace`.
///
/// This is `O(n + k log k)` instead of `O(n * k)` with repeated `Vec::splice`.
///
/// # Assumptions
/// - patches are sorted by [`sort_by_priority`].
fn apply_ops<'a>(
    mut base: Vec<Value<'a>>,
    patches: Vec<ValueWithPriority<'a>>,
) -> Result<Vec<Value<'a>>> {
    let base_len = base.len();
    let mut removed = vec![false; base_len];
    // (insert position in original indices, values)
    let mut inserts: Vec<(usize, Vec<Value<'a>>)> = Vec::new();
    // Values of `Replace` that overflow past the end. These are appended last.
    let mut overflow: Vec<Value<'a>> = Vec::new();

    for ValueWithPriority { patch, .. } in patches {
        let JsonPatch { action, value } = patch;

        match action {
            Action::Seq { op: Op::Replace, range } => {
                let (in_bounds, overflow_range) = split_range_at_len(range, base_len);
                let mut values = value_as_array(value)?.into_iter();

                if let Some(in_bounds) = in_bounds {
                    for index in in_bounds {
                        if let Some(value) = values.next() {
                            base[index] = value;
                            removed[index] = false;
                        } else {
                            // Fewer values than the range: the remaining elements are removed.
                            removed[index] = true;
                        }
                    }
                }

                // NOTE: If the range fits in bounds, surplus values are discarded (same as zip).
                if overflow_range.is_some() {
                    overflow.extend(values);
                }
            }
            Action::Seq { op: Op::Remove, range } => {
                let Some(flags) = removed.get_mut(range.clone()) else {
                    return Err(JsonPatchError::UnexpectedRange {
                        patch_range: range,
                        actual_len: base_len,
                    });
                };
                flags.fill(true);
            }
            Action::Seq { op: Op::Add, range } => {
                // Inserts past the end are appended in priority order.
                inserts.push((range.start.min(base_len), value_as_array(value)?));
            }
            Action::SeqPush => inserts.push((base_len, value_as_array(value)?)),
            Action::Pure { op: Op::Add } => {} // Ignored as before.
            unexpected @ Action::Pure { .. } => {
                return Err(JsonPatchError::ExpectedSeq { unexpected });
            }
        }
    }

    // NOTE: Must be a stable sort to keep the priority order at the same position.
    inserts.sort_by_key(|(at, _)| *at);

    let extra_len: usize = inserts.iter().map(|(_, values)| values.len()).sum();
    let mut patched = Vec::with_capacity(base_len + extra_len + overflow.len());
    let mut inserts = inserts.into_iter().peekable();

    for (index, (value, is_removed)) in base.into_iter().zip(removed).enumerate() {
        while let Some((_, values)) = inserts.next_if(|(at, _)| *at == index) {
            patched.extend(values);
        }
        if !is_removed {
            patched.push(value);
        }
    }
    patched.extend(inserts.flat_map(|(_, values)| values)); // Add past the end & SeqPush
    patched.extend(overflow);

    Ok(patched)
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
                simd_json::TryTypeError { expected: simd_json::ValueType::Array, got: value_type },
                &["".into()],
                other,
            ))
        }
    }
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
    let mut max_index = 0;
    let mut rows: Vec<TableRow> = patches
        .iter()
        .filter_map(|patch| {
            match &patch.patch.action {
                Action::Seq { op, range } => {
                    let action_type = match op {
                        Op::Add => ActionType::Add,
                        Op::Replace => ActionType::Replace,
                        Op::Remove => ActionType::Remove,
                    };
                    max_index = max_index.max(range.end);
                    Some(TableRow {
                        op: action_type,
                        priority: patch.priority,
                        range: range.clone(),
                    })
                }
                Action::SeqPush => {
                    let push_len =
                        simd_json::derived::ValueTryAsArray::try_as_array(&patch.patch.value)
                            .map_or(1, |a| a.len());

                    let start = target_array_len;
                    let end = start + push_len;
                    max_index = max_index.max(end);
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
    let points: Vec<_> = points.into_iter().collect();

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
        if let Some(prev) = last
            && seg.start > prev + GAP_THRESHOLD
        {
            header.push_str(&format!("{ELLIPSIS:^cell_width$}"));
        }

        if seg.end == seg.start + 1 {
            // single index
            header.push_str(&format!("{:^cell_width$}", seg.start));
        } else {
            // multiple indices
            header.push_str(&format!("{:^cell_width$}", format!("{}-{}", seg.start, seg.end - 1)));
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
            a.op.rank().cmp(&b.op.rank()).then(a.priority.cmp(&b.priority))
        };
        rows.sort_by(priority_sort);
    }

    // --- 7. render each row
    for row in &rows {
        out.push_str(&format!("{:<7} | {:>3} |", row.op.as_str(), row.priority));
        let mut last = None;
        for seg in &segments {
            if let Some(prev) = last
                && seg.start > prev + GAP_THRESHOLD
            {
                out.push_str(ELLIPSIS);
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
                action: Action::Seq { op: Op::Replace, range: 1..4 },
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
                    action: Action::Seq { op: Op::Add, range: 1..3 },
                    value: simd_json::json_typed! {borrowed, ["a", "b"]},
                },
                priority: 1,
            },
            // Replace a range
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq { op: Op::Replace, range: 4..6 },
                    value: simd_json::json_typed! {borrowed, ["x1", "x2"]},
                },
                priority: 0,
            },
            // Remove a range
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq { op: Op::Remove, range: 2..4 },
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

    fn seq(
        op: Op,
        range: core::ops::Range<usize>,
        value: Value<'static>,
        priority: usize,
    ) -> ValueWithPriority<'static> {
        ValueWithPriority {
            patch: JsonPatch { action: Action::Seq { op, range }, value },
            priority,
        }
    }

    /// Adds are applied at their original index even if a lower-priority add targets a later index.
    #[test]
    fn add_positions_are_independent_of_priority_order() {
        let patches = vec![
            seq(Op::Add, 10..10, json_typed!(borrowed, ["A", "A", "A"]), 0),
            seq(Op::Add, 2..2, json_typed!(borrowed, ["B"]), 1),
        ];

        let mut actual =
            json_typed!(borrowed, ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"]);
        apply_seq_array_directly(actual.try_as_array_mut().unwrap(), patches).unwrap();

        let expected = json_typed!(
            borrowed,
            ["0", "1", "B", "2", "3", "4", "5", "6", "7", "8", "9", "A", "A", "A", "10", "11"]
        );
        assert_eq!(actual, expected);
    }

    /// Overflowing replace with fewer values than its in-bounds part must not panic.
    #[test]
    fn overflowing_replace_with_few_values_does_not_panic() {
        let patches = vec![seq(Op::Replace, 4..10, json_typed!(borrowed, ["X"]), 0)];

        let mut actual = json_typed!(borrowed, ["0", "1", "2", "3", "4", "5"]);
        apply_seq_array_directly(actual.try_as_array_mut().unwrap(), patches).unwrap();

        let expected = json_typed!(borrowed, ["0", "1", "2", "3", "X"]);
        assert_eq!(actual, expected);
    }

    /// Overflow values of a replace are appended to the end.
    #[test]
    fn overflowing_replace_appends_rest() {
        let patches = vec![seq(Op::Replace, 4..7, json_typed!(borrowed, ["X", "Y", "Z", "W"]), 0)];

        let mut actual = json_typed!(borrowed, ["0", "1", "2", "3", "4", "5"]);
        apply_seq_array_directly(actual.try_as_array_mut().unwrap(), patches).unwrap();

        let expected = json_typed!(borrowed, ["0", "1", "2", "3", "X", "Y", "Z", "W"]);
        assert_eq!(actual, expected);
    }

    /// In-bounds replace with more values than its range keeps the old behavior (surplus is discarded).
    #[test]
    fn in_bounds_replace_discards_surplus_values() {
        let patches = vec![seq(Op::Replace, 1..3, json_typed!(borrowed, ["X", "Y", "Z"]), 0)];

        let mut actual = json_typed!(borrowed, ["0", "1", "2", "3", "4", "5"]);
        apply_seq_array_directly(actual.try_as_array_mut().unwrap(), patches).unwrap();

        let expected = json_typed!(borrowed, ["0", "X", "Y", "3", "4", "5"]);
        assert_eq!(actual, expected);
    }

    /// Same `(priority, op)` patches keep the input order (stable & deterministic).
    #[test]
    fn same_priority_adds_keep_input_order() {
        let patches: Vec<_> = (0..64)
            .map(|i| seq(Op::Add, 1..1, Value::Array(Box::new(vec![Value::from(i as u64)])), 7))
            .collect();

        let mut actual = json_typed!(borrowed, ["first", "last"]);
        apply_seq_array_directly(actual.try_as_array_mut().unwrap(), patches).unwrap();

        let mut expected = vec![Value::from("first")];
        expected.extend((0..64).map(|i| Value::from(i as u64)));
        expected.push(Value::from("last"));
        assert_eq!(actual, Value::Array(Box::new(expected)));
    }
}
