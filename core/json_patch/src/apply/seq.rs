#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
use crate::ptr_mut::PointerMut as _;
use crate::vec_utils::{SmartExtend as _, SmartIntoIter as _};
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
pub(crate) fn apply_seq_by_priority<'a>(
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
    tracing::debug!(
        "Seq merge conflict resolution for `{file_name}` file:
Path: {}
{}",
        path.join("/"),
        visualize_ops(&patches)
    );

    let patch_target_vec = core::mem::take(template_array);
    let patched_array = apply_ops_parallel(*patch_target_vec, patches)
        .smart_iter()
        .filter(|v| v != &MARK_AS_REMOVED);

    template_array.smart_extend(patched_array);

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

        let op_rank = |patch: &JsonPatch<'_>| match patch.op.as_seq().op {
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

        let op_rank = |patch: &JsonPatch<'_>| match patch.op.as_seq().op {
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
    base: Vec<Value<'a>>,
    patches: Vec<ValueWithPriority<'a>>,
) -> Vec<Value<'a>> {
    use std::sync::{Arc, Mutex};

    let (non_add_ops, add_ops): (Vec<_>, Vec<_>) = patches
        .smart_iter()
        .partition(|ValueWithPriority { patch, .. }| patch.op.as_seq().op != Op::Add);

    let base = Arc::new(Mutex::new(base));

    // Replace, Remove
    non_add_ops
        .smart_iter()
        .for_each(|ValueWithPriority { patch, .. }| {
            let seq = patch.op.as_seq();
            match seq.op {
                Op::Replace => {
                    let values = patch.value;
                    seq.range
                        .clone()
                        .smart_iter()
                        .zip(values.try_into_array().expect("array"))
                        .for_each(|(i, v)| {
                            let mut base = base.lock().unwrap();
                            if i < base.len() {
                                base[i] = v;
                            }
                        });
                }
                Op::Remove => {
                    seq.range.clone().smart_iter().for_each(|i| {
                        let mut base = base.lock().unwrap();
                        if i < base.len() {
                            base[i] = MARK_AS_REMOVED;
                        }
                    });
                }
                Op::Add => {}
            }
        });

    // Add
    let mut base = Arc::try_unwrap(base)
        .expect("No other Arc references")
        .into_inner()
        .unwrap();
    let mut offset = 0;
    for value in add_ops {
        let ValueWithPriority { patch, .. } = value;
        let seq = patch.op.as_seq();
        let values = patch.value.try_into_array().expect("array");

        let insert_at = seq.range.start + offset;
        let len = values.len();
        if insert_at <= base.len() {
            base.splice(insert_at..insert_at, values);
            offset += len;
        } else {
            base.smart_extend(values);
        }
    }

    base
}

#[cfg(any(feature = "tracing", test))]
fn visualize_ops(patches: &[ValueWithPriority<'_>]) -> String {
    const CELL_WIDTH: usize = 4;
    const OMITTED_COUNT: usize = 10;

    let width = patches
        .smart_iter()
        .map(|v| v.patch.op.as_seq().range.end)
        .max()
        .unwrap_or(0);

    let mut output = String::new();

    // Index row
    for i in 1..=width {
        if width > OMITTED_COUNT * 2 && i > OMITTED_COUNT && i <= width - OMITTED_COUNT {
            if i == OMITTED_COUNT + 1 {
                output.push_str(" ... ");
            }
            continue;
        }
        output.push_str(&format!("{i:^width$}", width = CELL_WIDTH));
    }
    output.push_str("| Op      | priority |\n");

    for value in patches {
        let priority = value.priority;
        let seq = value.patch.op.as_seq();
        let op = seq.op;
        let range = seq.range.clone();

        for i in 1..=width {
            if width > OMITTED_COUNT * 2 && i > OMITTED_COUNT && i <= width - OMITTED_COUNT {
                if i == OMITTED_COUNT + 1 {
                    output.push_str(" ... ");
                }
                continue;
            }

            let symbol = if i > range.start && i <= range.end {
                match op {
                    Op::Add => "[+] ",
                    Op::Replace => "[*] ",
                    Op::Remove => "[-] ",
                }
            } else {
                "    "
            };
            output.push_str(symbol);
        }

        let label = format!(
            "| {:<7} | {priority:>8} |",
            match op {
                Op::Add => "Add",
                Op::Remove => "Remove",
                Op::Replace => "Replace",
            },
        );
        output.push_str(&label);
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpRange, OpRangeKind};

    fn draw_box(data: &[Value<'_>]) -> String {
        data.smart_iter()
            .enumerate()
            .map(|(idx, v)| {
                let mut s = String::new();

                if idx % 20 == 0 {
                    if idx != 0 {
                        s.push('\n');
                    }
                    s.push_str("    ");
                }

                match v {
                    v if v == &MARK_AS_REMOVED => s.push_str("[ ]"),
                    _ => s.push_str("[{}]"),
                };

                s
            })
            .collect()
    }

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
        let base_seq: Vec<Value<'_>> = base_seq.smart_iter().map(|i| i.into()).collect();

        sort_by_priority(&mut patches);
        println!("Operation Map:\n{}", visualize_ops(&patches));
        let result = apply_ops_parallel(base_seq, patches);

        println!("\nFinal Result:");
        println!("{}", draw_box(&result));

        let result: Vec<_> = result
            .smart_iter()
            .filter(|v| v != &MARK_AS_REMOVED)
            .collect();
        println!("{result:#?}");
    }
}
