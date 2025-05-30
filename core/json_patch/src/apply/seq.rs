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
    json: &mut Value<'a>,
    path: JsonPath<'a>,
    mut patches: Vec<ValueWithPriority<'a>>,
) -> Result<()> {
    let target = json
        .ptr_mut(&path)
        .ok_or_else(|| JsonPatchError::NotFoundTarget {
            path: path.join("."),
        })?;

    let Value::Array(template_array) = target else {
        return Err(JsonPatchError::UnsupportedRangeKind);
    };

    sort_by_priority(patches.as_mut_slice());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OpRange, OpRangeKind};

    fn visualize_ops(ops: &[ValueWithPriority<'_>], width: usize) -> String {
        let label_start: usize = 60;

        let mut s = String::new();
        s.push_str(&format!("Base index: [0..{}]\n", width - 1));

        for value in ops {
            let ValueWithPriority { patch, priority } = value;

            let mut line = vec![' '; label_start + 40];

            let seq = patch.op.as_seq();
            let op = seq.op;
            let range = seq.range.clone();

            let start = range.start.min(width);
            let end = range.end.min(width);
            let label = format!(
                "| op: {:<7} | priority({})",
                match op {
                    Op::Add => "add",
                    Op::Remove => "remove",
                    Op::Replace => "replace",
                },
                priority
            );

            for i in start..end {
                let idx = i * 3;
                if idx + 2 < label_start {
                    line[idx] = '[';
                    line[idx + 1] = '-';
                    line[idx + 2] = ']';
                }
            }

            for (i, c) in label.chars().enumerate() {
                if label_start + i < line.len() {
                    line[label_start + i] = c;
                }
            }
            line.push('\n');
            s.push_str(&line.smart_iter().collect::<String>());
        }

        s
    }

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
        println!("Operation Map:\n{}", visualize_ops(&patches, 21));
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
