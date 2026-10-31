use std::{borrow::Cow, ops::Range};

use json_patch::{JsonPath, ValueWithPriority};
use winnow::{
    Parser as _,
    ascii::multispace0,
    combinator::{alt, opt, repeat},
    error::{StrContext::*, StrContextValue::*},
};
use winnow_ext::ReadableError;

use super::ClipAnimDiffPatch;
use crate::{
    adsf::patch::de::error::{Error, Result},
    common_parser::{
        delete_line::delete_this_line,
        lines::{one_line, verify_line_parses_to},
    },
};

/// A raw Nemesis diff block captured during parsing.
///
/// This structure does not interpret the diff content.
/// It only records where (path) and what (raw text) was modified.
#[derive(Debug, Clone)]
pub(super) struct RawDiff<'a> {
    /// Array end push Operation?
    pub op: Op,

    /// The index of the array being processed. Used for `range.start` in the patch.
    pub seq_index: Option<usize>,

    /// The number of lines in the comment labeled `<-- ORIGINAL -->`.
    /// This is used for `range.end` when calling `Remove`.
    pub original_len: Option<usize>,

    /// `JsonPath` at the moment the diff block starts.
    pub path: JsonPath<'a>,

    /// Raw text slice covering `OPEN` to `ORIGINAL`/`CLOSE` (exclusive of the comments).
    pub text: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Op {
    Add,
    Replace,
    Remove,
    /// Array push.
    SeqPush,
}

pub(crate) fn into_patch_map(
    raw_diffs: Vec<RawDiff<'_>>,
    priority: usize,
) -> Result<ClipAnimDiffPatch<'_>> {
    let mut patches = ClipAnimDiffPatch::default();

    for raw in raw_diffs {
        let last = raw.path.last().map(|s| s.as_ref());
        let op = raw.op;

        match (op, last) {
            // -------------------------
            // Scalar fields (always op::replace)
            // -------------------------
            (_, Some("name")) => {
                patches.name = Some(
                    one_line
                        .context(Expected(Description("name: str")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }
            (_, Some("clip_id")) => {
                patches.clip_id = Some(
                    one_line
                        .context(Expected(Description("clip_id: str")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }
            (_, Some("play_back_speed")) => {
                patches.play_back_speed = Some(
                    verify_line_parses_to::<f32>
                        .context(Expected(Description("play_back_speed: f32")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }
            (_, Some("crop_start_local_time")) => {
                patches.crop_start_local_time = Some(
                    verify_line_parses_to::<f32>
                        .context(Expected(Description("crop_start_local_time: f32")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }
            (_, Some("crop_end_local_time")) => {
                patches.crop_end_local_time = Some(
                    verify_line_parses_to::<f32>
                        .context(Expected(Description("crop_end_local_time: f32")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }

            // -------------------------
            // trigger_names sequence
            // -------------------------
            (Op::Remove, None) => {
                let start = raw.seq_index.ok_or_else(|| Error::NotFoundApplyTarget {
                    kind: "trigger_names remove requires seq_index".to_owned(),
                })?;
                let original_len = raw.original_len.ok_or_else(|| Error::NotFoundApplyTarget {
                    kind: "trigger_names remove requires original_len".to_owned(),
                })?;

                patches.trigger_names_patches.seq.push(ValueWithPriority {
                    patch: json_patch::JsonPatch {
                        action: json_patch::Action::Seq {
                            op: json_patch::Op::Remove,
                            range: Range { start, end: start + original_len },
                        },
                        value: simd_json::BorrowedValue::Static(simd_json::StaticNode::Null),
                    },
                    priority,
                });
            }
            (_, last) => {
                // There are two variations of this pattern.
                // - A diff that changes only `trigger_names_len`.
                // - A diff that replaces both `trigger_names_len` and `trigger_names`.
                let input = if last == Some("trigger_names_len") {
                    let mut input = raw.text;
                    verify_line_parses_to::<usize>
                        .context(Expected(Description("trigger_names_len: usize")))
                        .parse_next(&mut input)
                        .map_err(|err| {
                            ReadableError::from_display(err, raw.text, raw.text.len() - input.len())
                        })?;

                    // If `len` is the only parameter, the patch consists only of the
                    // difference in `len`. Since `len` can be derived from `Vec::len`,
                    // this is unnecessary and can be skipped.
                    if input.trim().is_empty() {
                        continue;
                    }

                    input
                } else {
                    raw.text
                };

                let (v, delete_len) =
                    trigger_names.parse(input).map_err(|e| ReadableError::from_parse(e))?;
                let action =
                    make_seq_action(op, raw.seq_index, raw.original_len, v.len() + delete_len);

                patches.trigger_names_patches.seq.push(ValueWithPriority {
                    patch: json_patch::JsonPatch { action, value: v.into() },
                    priority,
                });
            }
        }
    }

    Ok(patches)
}

const fn make_seq_action(
    op: Op,
    seq_index: Option<usize>,
    original_len: Option<usize>,
    new_len: usize,
) -> json_patch::Action {
    match (seq_index, original_len) {
        (Some(start), _) => json_patch::Action::Seq {
            op: to_json_patch_op(op),
            range: Range { start, end: start + new_len },
        },
        // Diff starts from `trigger_names_len`, meaning the whole array is replaced.
        (None, Some(_)) => json_patch::Action::Seq {
            op: to_json_patch_op(op),
            range: Range { start: 0, end: new_len },
        },
        (None, None) => json_patch::Action::SeqPush,
    }
}

const fn to_json_patch_op(op: Op) -> json_patch::Op {
    match op {
        Op::Replace => json_patch::Op::Replace,
        Op::Remove => json_patch::Op::Remove,
        Op::Add | Op::SeqPush => json_patch::Op::Add, // unreachable Op::SeqPush
    }
}

/// Parse `<trigger_name: str>\n` or `//* delete this line *//`.
fn trigger_names<'a>(input: &mut &'a str) -> winnow::ModalResult<(Vec<Cow<'a, str>>, usize)> {
    multispace0.parse_next(input)?;

    let mut delete_count = 0;
    while opt(delete_this_line).parse_next(input)?.is_some() {
        delete_count += 1;
    }

    let diff = repeat(
        1..,
        alt((
            delete_this_line.map(|_| {
                delete_count += 1;
                None
            }),
            one_line.context(Expected(Description("trigger_name: str"))).map(Some),
        )),
    )
    .map(|items: Vec<Option<Cow<str>>>| items.into_iter().flatten().collect::<Vec<_>>())
    .context(Label("TriggerNamesDiff"))
    .parse_next(input)?;

    while opt(delete_this_line).parse_next(input)?.is_some() {
        delete_count += 1;
    }
    multispace0.parse_next(input)?;

    Ok((diff, delete_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_names_parses_delete_lines() {
        let input = r"
//* delete this line *//
//* delete this line *//
clipStart:1.0
//* delete this line *//
clipEnd:2.0
//* delete this line *//
";

        let (diff, delete_count) = trigger_names.parse(input).unwrap();

        assert_eq!(delete_count, 4);
        assert_eq!(diff.len(), 2);
        assert_eq!(diff[0], "clipStart:1.0");
        assert_eq!(diff[1], "clipEnd:2.0");
    }

    #[test]
    fn trigger_names_without_delete_lines() {
        let input = "\nclipStart:1.0\n";

        let (diff, delete_count) = trigger_names.parse(input).unwrap();

        assert_eq!(delete_count, 0);
        assert_eq!(diff.len(), 1);
    }
}
