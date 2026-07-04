use std::{borrow::Cow, ops::Range};

use json_patch::{JsonPath, ValueWithPriority};
use winnow::{
    ModalResult, Parser as _,
    ascii::{float, multispace0, space0, space1},
    combinator::{opt, repeat},
    error::{StrContext::*, StrContextValue::*},
};
use winnow_ext::ReadableError;

use crate::{
    adsf::{
        normal::{Rotation, Translation},
        patch::{ClipMotionDiffPatch, de::others::clip_motion::deserializer::ArrayType},
    },
    asdsf::patch::de::error::Result,
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
    /// Parsed category.
    ///
    /// But that doesn't necessarily mean it's correct.
    /// For example, an addition at the end of a category might actually be a diff for the next category.
    pub seq_type: ArrayType,

    /// Array end push Operation?
    pub op: Op,

    /// The index of the array being processed. Used for range.start in .patch.
    pub seq_index: Option<usize>,

    /// The number of lines in the comment labeled `<-- ORIGINAL -->`.
    /// This is used for `range.end` when calling `Remove`.
    ///
    /// This is to prevent confusion about how many lines to delete.
    pub original_len: Option<usize>,

    /// JsonPath at the moment the diff block starts.
    pub path: JsonPath<'a>,

    /// Raw text slice covering `OPEN` to `ORIGINAL`/`CLOSE` (inclusive).
    pub text: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Op {
    Add,
    Replace,
    Remove,
    /// ArrayPush.
    SeqPush,
}

pub(crate) fn into_patch_map(
    raw_diffs: Vec<RawDiff<'_>>,
    priority: usize,
) -> Result<ClipMotionDiffPatch<'_>> {
    let mut patches = ClipMotionDiffPatch::default();

    for raw in raw_diffs {
        let last = raw.path.last().map(|s| s.as_ref());
        let op = raw.op;

        match (raw.seq_type, op, last) {
            // -------------------------
            // clip_id / duration
            // -------------------------
            (_, _, Some("clip_id")) => {
                patches.clip_id = Some(
                    one_line
                        .context(Expected(Description("clip_id: str")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }

            (_, _, Some("duration")) => {
                patches.duration = Some(
                    verify_line_parses_to::<f32>
                        .context(Expected(Description("duration: f32")))
                        .context(Label("Diff"))
                        .parse(raw.text)
                        .map_err(|e| ReadableError::from_parse(e))?,
                );
            }

            // -------------------------
            // Translation sequence
            // -------------------------
            (ArrayType::Translation, Op::Remove, None) => {
                let start = raw.seq_index.ok_or_else(|| {
                    crate::asdsf::patch::de::error::Error::NotFoundApplyTarget {
                        kind: "translation remove requires seq_index".to_owned(),
                    }
                })?;

                let original_len = raw.original_len.ok_or_else(|| {
                    crate::asdsf::patch::de::error::Error::NotFoundApplyTarget {
                        kind: "translation remove requires original_len".to_owned(),
                    }
                })?;

                patches.translations_patches.seq.push(ValueWithPriority {
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
            (ArrayType::Translation, _, None) => {
                let (v, delete_len) =
                    translation.parse(raw.text).map_err(|e| ReadableError::from_parse(e))?;

                let range =
                    raw.seq_index.map(|start| Range { start, end: start + v.len() + delete_len });

                let action = match range {
                    None => json_patch::Action::SeqPush,
                    Some(range) => json_patch::Action::Seq { op: to_json_patch_op(op), range },
                };

                patches.translations_patches.seq.push(ValueWithPriority {
                    patch: json_patch::JsonPatch { action, value: v.into() },
                    priority,
                });
            }

            // -------------------------
            // Rotation sequence
            // -------------------------
            (ArrayType::Rotation, Op::Remove, None) => {
                let start = raw.seq_index.ok_or_else(|| {
                    crate::asdsf::patch::de::error::Error::NotFoundApplyTarget {
                        kind: "rotation remove requires seq_index".to_owned(),
                    }
                })?;

                let original_len = raw.original_len.ok_or_else(|| {
                    crate::asdsf::patch::de::error::Error::NotFoundApplyTarget {
                        kind: "rotation remove requires original_len".to_owned(),
                    }
                })?;

                patches.rotations_patches.seq.push(ValueWithPriority {
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
            (ArrayType::Rotation, _, None) => {
                let (v, delete_len) =
                    rotation.parse(raw.text).map_err(|e| ReadableError::from_parse(e))?;

                let range =
                    raw.seq_index.map(|start| Range { start, end: start + v.len() + delete_len });

                let action = match range {
                    None => json_patch::Action::SeqPush,
                    Some(range) => json_patch::Action::Seq { op: to_json_patch_op(op), range },
                };

                patches.rotations_patches.seq.push(ValueWithPriority {
                    patch: json_patch::JsonPatch { action, value: v.into() },
                    priority,
                });
            }

            failure => {
                #[cfg(feature = "tracing")]
                tracing::debug!("unknown pattern: {failure:?}");

                return Err(crate::asdsf::patch::de::error::Error::NotFoundApplyTarget {
                    kind: format!("unknown pattern: {failure:?}"),
                });
            }
        }
    }

    Ok(patches)
}

/// # Panics
/// `SeqPush`
const fn to_json_patch_op(op: Op) -> json_patch::Op {
    match op {
        Op::Add => json_patch::Op::Add,
        Op::Replace => json_patch::Op::Replace,
        Op::Remove => json_patch::Op::Remove,
        Op::SeqPush => unreachable!(),
    }
}

/// parse `<time: f32> <text: str>\n` or `//* delete this line *//`
fn translation<'a>(input: &mut &'a str) -> ModalResult<(Vec<Translation<'a>>, usize)> {
    multispace0.parse_next(input)?;
    let mut delete_count = 0;
    while opt(delete_this_line).parse_next(input)?.is_some() {
        delete_count += 1;
    }

    let diff=     repeat(
        1..,
        winnow::seq! {
            Translation {
                _: space0,
                time: float::<_, f32, _>.take().map(Cow::Borrowed).context(Expected(Description("time: f32"))),
                _: space1,
                text: one_line.context(Expected(Description("text: str"))),
                _: multispace0,
            }
        }
        .context(Label("TranslationDiff")),
    )
    .context(Label("TranslationsDiff"))
    .parse_next(input)?;

    while opt(delete_this_line).parse_next(input)?.is_some() {
        delete_count += 1;
    }
    multispace0.parse_next(input)?;

    Ok((diff, delete_count))
}

/// parse `<time: f32> <text: str>\n` or `//* delete this line *//`
fn rotation<'a>(input: &mut &'a str) -> ModalResult<(Vec<Rotation<'a>>, usize)> {
    multispace0.parse_next(input)?;
    let mut delete_count = 0;
    while opt(delete_this_line).parse_next(input)?.is_some() {
        delete_count += 1;
    }

    let diff = repeat(
        1..,
        winnow::seq! {
            Rotation {
                _: space0,
                time: float::<_, f32, _>.take().map(Cow::Borrowed).context(Expected(Description("time: f32"))),
                _: space1,
                text: one_line.context(Expected(Description("text: str"))),
                _: multispace0,
            }
        }
        .context(Label("RotationDiff")),
    )
    .context(Label("RotationsDiff"))
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
    fn translation_parses_delete_lines() {
        let input = r"
//* delete this line *//
//* delete this line *//
0.10 1 2 3
0.20 4 5 6
//* delete this line *//
";

        let (diff, delete_count) = translation.parse(input).unwrap();

        assert_eq!(delete_count, 3);

        assert_eq!(diff.len(), 2);

        assert_eq!(diff[0].time, "0.10");
        assert_eq!(diff[0].text, "1 2 3");

        assert_eq!(diff[1].time, "0.20");
        assert_eq!(diff[1].text, "4 5 6");
    }

    #[test]
    fn rotation_parses_delete_lines() {
        let input = r"
//* delete this line *//
0.30 7 8 9 10
0.40 11 12 13 14
//* delete this line *//
//* delete this line *//
";

        let (diff, delete_count) = rotation.parse(input).unwrap();

        assert_eq!(delete_count, 3);

        assert_eq!(diff.len(), 2);

        assert_eq!(diff[0].time, "0.30");
        assert_eq!(diff[0].text, "7 8 9 10");

        assert_eq!(diff[1].time, "0.40");
        assert_eq!(diff[1].text, "11 12 13 14");
    }

    #[test]
    fn translation_without_delete_lines() {
        let input = r"
0.10 1 2 3
";

        let (diff, delete_count) = translation.parse(input).unwrap();

        assert_eq!(delete_count, 0);
        assert_eq!(diff.len(), 1);
    }

    #[test]
    fn rotation_without_delete_lines() {
        let input = r"
0.10 1 2 3 4
";

        let (diff, delete_count) = rotation.parse(input).unwrap();

        assert_eq!(delete_count, 0);
        assert_eq!(diff.len(), 1);
    }
}
