use json_patch::JsonPath;
use winnow::{
    Parser,
    ascii::{multispace0, space1},
    combinator::opt,
    error::{ContextError, ErrMode, StrContext, StrContextValue},
};
use winnow_ext::ReadableError;

use super::raw_diff::RawDiff;
use crate::{
    adsf::patch::{
        ClipMotionDiffPatch,
        de::{
            error::{Error, Result},
            others::clip_motion::raw_diff::Op,
        },
    },
    common_parser::{
        comment::close_comment_line,
        lines::{one_line, parse_one_line},
    },
};

/// Parse `animationdatasinglefile.txt` clip motion patch.
///
/// # Errors
/// Parse failed.
pub fn parse_clip_motion_diff_patch(
    adsf_patch: &str,
    priority: usize,
) -> Result<ClipMotionDiffPatch<'_>> {
    let mut patcher_de = PatchDeserializer::new(adsf_patch);
    patcher_de.root_class().map_err(|err| patcher_de.to_readable_err(err))?;

    super::raw_diff::into_patch_map(patcher_de.raw_diffs, priority)
}

/// Nemesis patch deserializer
#[derive(Debug, Clone)]
struct PatchDeserializer<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// This is readonly for error report. Not move position.
    original: &'a str,

    /// Raw diff blocks captured during parsing.
    raw_diffs: Vec<RawDiff<'a>>,

    ///When an `ORIGINAL` comment arrives,
    /// we need to parse it for the number of len elements, but we don't treat it as a diff until CLOSE.
    /// This is the flag for that purpose.
    ignore_close: bool,

    /// Indicates the current json position during one patch file.
    ///
    /// e.g. `["attack", "[9]", "clip_names", "clip_name"]`
    path: JsonPath<'a>,

    /// Array end push Operation?
    op: Op,

    /// Parsed category.
    ///
    /// But that doesn't necessarily mean it's correct.
    /// For example, an addition at the end of a category might actually be a diff for the next category.
    category: ArrayType,

    /// The index of the array being processed.
    /// Since patching the entire array does not output the index to path, we store it here.
    seq_index: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ArrayType {
    /// `Array<Translation>`
    Translation,

    /// `Array<Rotation>`
    Rotation,
}

impl<'de> PatchDeserializer<'de> {
    const fn new(input: &'de str) -> Self {
        Self {
            input,
            original: input,
            raw_diffs: Vec::new(),
            path: JsonPath::new(),
            op: Op::Add,
            ignore_close: false,
            category: ArrayType::Translation,
            seq_index: None,
        }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Parser methods

    fn parse_next<O>(
        &mut self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O> {
        parser.parse_next(&mut self.input).map_err(|err| Error::Context { err })
    }

    /// Convert Visitor errors to position-assigned errors.
    ///
    /// # Why is this necessary?
    /// Because Visitor errors that occur within each `Deserialize` implementation cannot indicate the error location in XML.
    #[cold]
    fn to_readable_err(&self, err: Error) -> Error {
        let readable = match err {
            Error::Context { err } => ReadableError::from_context(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
            Error::Readable { source } => source,
            err => ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        };

        Error::Readable { source: readable }
    }

    #[cold]
    fn message_to_readable_err(&self, err: impl core::fmt::Display) -> Error {
        Error::Readable {
            source: ReadableError::from_display(
                err,
                self.original,
                self.original.len() - self.input.len(),
            ),
        }
    }

    /// Capture a raw diff block if present at the current position.
    ///
    /// The diff block is associated with the current JsonPath.
    ///
    /// # Return
    /// Got diff?
    fn maybe_capture_diff(&mut self) -> Result<bool> {
        if self.ignore_close && self.parse_next(opt(close_comment_line))?.is_some() {
            self.ignore_close = false;
        };

        let Some((raw, has_original, original_len)) = self.parse_next(take_raw_diff)? else {
            return Ok(false);
        };

        let path = self.path.clone();

        let op = match (has_original, raw.is_empty()) {
            (true, true) => Op::Remove,
            (true, false) => Op::Replace,
            (false, true) => Op::Add,
            (false, false) => self.op,
        };

        if has_original {
            self.ignore_close = true;
        }

        self.raw_diffs.push(RawDiff {
            path,
            text: raw,
            op,
            seq_index: self.seq_index,
            seq_type: self.category,
            original_len,
        });
        Ok(true)
    }

    fn parse_array(&mut self, inner_type: ArrayType) -> Result<()> {
        self.category = inner_type;

        match inner_type {
            ArrayType::Translation => self.path.push("translations_len".into()),
            ArrayType::Rotation => self.path.push("rotations_len".into()),
        }
        let len = self.parse_len_line()?;
        self.path.pop();

        let mut index = 0;
        self.seq_index = Some(index);

        while index < len {
            if self.maybe_capture_diff()? {
                continue;
            }
            if let Err(Error::Context { .. }) = self.parse_annotation() {
                let array_target = match inner_type {
                    ArrayType::Translation => "Translations",
                    ArrayType::Rotation => "Rotations",
                };
                return Err(self.message_to_readable_err(format!(
                    "Invalid {array_target}.\n\nExpected {len} vanilla entries, but found only {index}."
                )));
            }

            index += 1;
            self.seq_index = Some(index);
        }

        // capture array push
        self.op = Op::SeqPush;
        self.seq_index = None;
        self.maybe_capture_diff()?;
        self.op = Op::Add;

        Ok(())
    }

    /// Any length info from 1 line.
    fn parse_len_line(&mut self) -> Result<usize> {
        use winnow::error::{StrContext::Expected, StrContextValue::Description};

        self.maybe_capture_diff()?;
        let len = self.parse_next(
            parse_one_line::<usize>.context(Expected(Description("length_line: usize"))),
        )?;
        #[cfg(feature = "tracing")]
        tracing::trace!("{:?}, line Length = {len}", self.path);
        Ok(len)
    }

    /// Parse 1 line(but ignore new line)
    fn parse_str_line(&mut self) -> Result<()> {
        self.maybe_capture_diff()?;
        let _s = self.parse_next(one_line)?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_s);

        Ok(())
    }

    fn parse_line_f32(&mut self) -> Result<()> {
        self.maybe_capture_diff()?;
        let _value = self.parse_next(parse_one_line::<f32>.context(
            winnow::error::StrContext::Expected(winnow::error::StrContextValue::Description("f32")),
        ))?;
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_value);

        Ok(())
    }

    /// parse `<time: f32> <text: str>\n`
    fn parse_annotation(&mut self) -> Result<()> {
        self.parse_next(
            winnow::ascii::float::<_, f32, _>
                .context(StrContext::Expected(StrContextValue::Description("f32"))),
        )?;
        self.parse_next(space1.context(StrContext::Expected(StrContextValue::Description(
            "space between time & text",
        ))))?;

        let _s = self.parse_next(winnow::ascii::till_line_ending)?;
        // In the case of patches, this may not be present, so `opt`
        self.parse_next(opt(winnow::ascii::line_ending))?; // skip line end
        #[cfg(feature = "tracing")]
        tracing::debug!(?self.path, ?_s);

        Ok(())
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 asdsf patch
    fn root_class(&mut self) -> Result<()> {
        self.parse_next(multispace0)?;
        self.parse_clip_id()?;
        self.parse_duration()?;

        self.parse_array(ArrayType::Translation)?; // triggers
        self.parse_array(ArrayType::Rotation)?;

        #[cfg(feature = "tracing")]
        tracing::debug!("{:#?}", self.raw_diffs);
        Ok(())
    }

    fn parse_clip_id(&mut self) -> Result<()> {
        self.path.push("clip_id".into());
        self.parse_str_line()?; // e.g., 100, $aaaa
        self.path.pop();

        Ok(())
    }

    fn parse_duration(&mut self) -> Result<()> {
        self.path.push("duration".into());
        self.parse_line_f32()?;
        self.path.pop();

        Ok(())
    }
}

/// Returns
/// (diff, has_original, original_len)
fn take_raw_diff<'a>(
    input: &mut &'a str,
) -> winnow::ModalResult<Option<(&'a str, bool, Option<usize>)>> {
    use crate::common_parser::comment::{open_comment, take_till_close, take_till_original};

    // Fast path: no OPEN comment ahead
    if opt(open_comment).parse_next(input)?.is_none() {
        return Ok(None);
    }
    #[cfg(feature = "tracing")]
    tracing::debug!("Open diff");

    // take_till_original may overshoot into the *next* diff's ORIGINAL
    // when this diff has no ORIGINAL of its own
    // (pattern: OPEN -> CLOSE -> OPEN -> ORIGINAL).
    // Comparing remaining lengths tells us which marker truly comes first.
    let (original_remain, original_found) = opt(take_till_original).parse_peek(input)?;
    let (close_remain, close_found) = opt(take_till_close).parse_peek(input)?;

    let has_original = match (original_found, close_found) {
        // ORIGINAL belongs to this diff only if reaching it consumed
        // *less* input than reaching CLOSE did, i.e. it comes first.
        (Some(_), Some(_)) => original_remain.len() > close_remain.len(),
        (Some(_), None) => true,
        _ => false,
    };

    let (diff, has_original, original_len) = if has_original {
        let diff = take_till_original.parse_next(input)?;
        let (_, original) = take_till_close.parse_peek(input)?;
        let original_len = original.lines().filter(|l| !l.trim().is_empty()).count();
        (diff, true, Some(original_len))
    } else {
        let diff = take_till_close.parse_next(input)?;
        (diff, false, None)
    };

    #[cfg(feature = "tracing")]
    tracing::debug!(?diff, has_original, original_len);

    Ok(Some((diff, has_original, original_len)))
}

#[cfg(test)]
mod tests {
    use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        adsf::patch::de::others::clip_motion::ClipMotionDiffPatch,
        asdsf::patch::de::NonNestedArrayDiff,
    };

    // #[quick_tracing::init]
    #[test]
    fn test_patch_replace_translations() {
        // 100                    // clip_id
        // <!-- ORIGINAL -->
        // 93                     // clip_id
        // <!-- CLOSE -->
        // 2                      // duration
        // 1                      // transition len
        // 2 0 0 0                // transition
        // 1                      // rotation len
        // <!-- MOD_CODE ~test~ OPEN -->
        // 6 0 0 0 5              // rotation
        // 6 0 0 0 5              // rotation
        // 6 0 0 0 5              // rotation
        // <!-- ORIGINAL -->
        // 2 0 0 0 1              // rotation
        // <!-- CLOSE -->

        let input = "
        <!-- MOD_CODE ~test~ OPEN -->
100
<!-- ORIGINAL -->
93
<!-- CLOSE -->
2
1
2 0 0 0
1
<!-- MOD_CODE ~test~ OPEN -->
6 0 0 0 5
6 0 0 0 5
6 0 0 0 5
<!-- ORIGINAL -->
2 0 0 0 1
<!-- CLOSE -->
";

        let patches = parse_clip_motion_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipMotionDiffPatch {
            clip_id: Some("100".into()),
            duration: None,
            translations_patches: NonNestedArrayDiff::default(),
            rotations_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Seq { op: Op::Replace, range: 0..3 },
                        value: simd_json::json_typed!(borrowed, [
                            {
                                "time": "6",
                                "text": "0 0 0 5",
                            },
                            {
                                "time": "6",
                                "text": "0 0 0 5",
                            },
                            {
                                "time": "6",
                                "text": "0 0 0 5",
                            },
                        ]),
                    },
                    priority: 0,
                }],
            },
        };

        assert_eq!(patches, expected);
    }

    #[test]
    fn test_patch_ignore_translation_len_only() {
        let input = "
100
2
<!-- MOD_CODE ~test~ OPEN -->
3
<!-- ORIGINAL -->
2
0 0 0 0 1
0 0 0 0 1
<!-- CLOSE -->
2
0 0 0 0 1
0 0 0 0 1
";

        let patches = parse_clip_motion_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));

        let expected = ClipMotionDiffPatch {
            clip_id: None,
            duration: None,
            translations_patches: NonNestedArrayDiff::default(),
            rotations_patches: NonNestedArrayDiff::default(),
        };
        assert_eq!(patches, expected);
    }

    /// These are the requirements for supporting `Nemesis tkuc patch`.
    #[test]
    fn test_patch_continues_add_replace_translation() {
        let input = "
200
0.5
1
<!-- MOD_CODE ~test~ OPEN -->
0.25 10 20 30
0.75 40 50 60
<!-- CLOSE -->
<!-- MOD_CODE ~test~ OPEN -->
1.25 70 80 90
1.75 100 110 120
<!-- ORIGINAL -->
1.50 11 22 33
<!-- CLOSE -->
1
9.99 7 8 9 10
";

        let patches = parse_clip_motion_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));

        let expected = ClipMotionDiffPatch {
            clip_id: None,
            duration: None,
            translations_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![
                    ValueWithPriority {
                        patch: JsonPatch {
                            action: Action::Seq { op: Op::Add, range: 0..2 },
                            value: simd_json::json_typed!(borrowed, [
                                {
                                    "time": "0.25",
                                    "text": "10 20 30",
                                },
                                {
                                    "time": "0.75",
                                    "text": "40 50 60",
                                },
                            ]),
                        },
                        priority: 0,
                    },
                    ValueWithPriority {
                        patch: JsonPatch {
                            // NOTE: It is intentional that this range starts at 0.
                            // This is because the patch must be a range relative to the vanilla data.
                            // Internally, when resolving conflicts between Seq patches,
                            // the system first performs a `Replace` using the `Replace/Remove` marker, then an `Add/Push,` and finally the actual `Remove.`
                            // Therefore, it works without any issues even if patch creator intended for the `add` to start at 0.
                            // See `json_patch::apply::seq.rs`
                            action: Action::Seq { op: Op::Replace, range: 0..2 },
                            value: simd_json::json_typed!(borrowed, [
                                {
                                    "time": "1.25",
                                    "text": "70 80 90",
                                },
                                {
                                    "time": "1.75",
                                    "text": "100 110 120",
                                },
                            ]),
                        },
                        priority: 0,
                    },
                ],
            },
            rotations_patches: NonNestedArrayDiff::default(),
        };

        assert_eq!(patches, expected);
    }

    /// These are the requirements for supporting `Precision Creatures (v2.4)`.
    #[test]
    fn test_patch_replace_translation_from_len_line() {
        let input = "
200
3
<!-- MOD_CODE ~test~ OPEN -->
2
0.25 10 20 30
0.75 40 50 60
<!-- ORIGINAL -->
1
0.50 1 2 3
<!-- CLOSE -->
1
9.99 7 8 9 10";
        let patches = parse_clip_motion_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));

        let expected = ClipMotionDiffPatch {
            clip_id: None,
            duration: None,
            translations_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Seq { op: Op::Replace, range: 0..2 },
                        value: simd_json::json_typed!(borrowed, [
                            {
                                "time": "0.25",
                                "text": "10 20 30",
                            },
                            {
                                "time": "0.75",
                                "text": "40 50 60",
                            },
                        ]),
                    },
                    priority: 0,
                }],
            },
            rotations_patches: NonNestedArrayDiff::default(),
        };

        assert_eq!(patches, expected);
    }

    // #[quick_tracing::init]
    #[test]
    fn test_patch_remove_rotations() {
        let input = "
99
<!-- MOD_CODE ~test~ OPEN -->
1.25
<!-- ORIGINAL -->
2
<!-- CLOSE -->
1
<!-- MOD_CODE ~test~ OPEN -->
0.43 0 0 0
0.50 0 0 0
1.32 0 0 0
<!-- ORIGINAL -->
2 0 0 0
<!-- CLOSE -->
4
2 0 0 0 1
<!-- MOD_CODE ~test~ OPEN -->
<!-- ORIGINAL -->
2 0 0 0 1
2 0 0 0 1
<!-- CLOSE -->
2 0 0 0 1
";

        let patches = parse_clip_motion_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipMotionDiffPatch {
            clip_id: None,
            duration: Some("1.25".into()),
            translations_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Seq { op: Op::Replace, range: 0..3 },
                        value: simd_json::json_typed!(borrowed, [
                            {
                                "time": "0.43",
                                "text": "0 0 0",
                            },
                            {
                                "time": "0.50",
                                "text": "0 0 0",
                            },
                            {
                                "time": "1.32",
                                "text": "0 0 0",
                            },
                        ]),
                    },
                    priority: 0,
                }],
            },
            rotations_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Seq { op: Op::Remove, range: 1..3 },
                        value: simd_json::json_typed!(borrowed, null),
                    },
                    priority: 0,
                }],
            },
        };

        assert_eq!(patches, expected);
    }

    #[test]
    fn test_delete_this_line_patch() {
        let input = "
152
3
9
<!-- MOD_CODE ~test~ OPEN -->
0.0 0 0 0
0.34 0 72 0
0.49 0 153 0
//* delete this line *//
//* delete this line *//
//* delete this line *//
//* delete this line *//
//* delete this line *//
//* delete this line *//
<!-- ORIGINAL -->
0.21 -0.372 5.61 0
0.44 0.395 25.98 0
0.57 0.425 36.30 0
0.77 -0.523 40.18 0
0.97 -1.81 34.40 0
1.11 -1.06 23.63 0
1.27 1.08 11.42 0
1.69 2.12 0.22 0
2.02 0.01 0.00 0
<!-- CLOSE -->
2
2 0 0 0 1
2 0 0 0 1
";

        let patches = parse_clip_motion_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipMotionDiffPatch {
            clip_id: None,
            duration: None,
            translations_patches: NonNestedArrayDiff {
                one: Default::default(),
                seq: vec![ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Seq { op: Op::Replace, range: 0..9 },
                        value: simd_json::json_typed!(borrowed, [
                            {
                                "time": "0.0",
                                "text": "0 0 0",
                            },
                            {
                                "time": "0.34",
                                "text": "0 72 0",
                            },
                            {
                                "time": "0.49",
                                "text": "0 153 0",
                            },
                        ]),
                    },
                    priority: 0,
                }],
            },
            rotations_patches: NonNestedArrayDiff::default(),
        };

        assert_eq!(patches, expected);
    }
}
