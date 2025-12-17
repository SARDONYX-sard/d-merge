use crate::{
    adsf::normal::de::from_word_and_space,
    common_parser::{
        comment::{open_comment, original_or_close_comment, take_till_close, CommentKind},
        lines::{one_line, verify_line_parses_to},
    },
};
use crate::{
    adsf::{
        normal::{Rotation, Translation},
        patch::de::{
            error::{Error, Result},
            others::clip_motion::{
                current_state::{CurrentState, PartialRotations, PartialTranslations},
                ClipMotionDiffPatch, DiffRotations, DiffTransitions, LineKind,
            },
        },
    },
    common_parser::delete_line::delete_this_line,
};
use json_patch::Op;
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::{line_ending, multispace0, till_line_ending},
    combinator::{eof, opt},
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    Parser,
};

/// Parse animationdatasinglefile.txt clip motion block patch.
///
/// # Errors
/// Parse failed.
pub fn parse_clip_motion_diff_patch(input: &str) -> Result<ClipMotionDiffPatch<'_>, Error> {
    let mut deserializer = Deserializer::new(input);
    deserializer
        .root()
        .map_err(|err| deserializer.to_readable_err(err))?;
    Ok(deserializer.output_patches)
}

/// Nemesis patch deserializer
#[derive(Debug)]
struct Deserializer<'a> {
    /// mutable pointer to str
    input: &'a str,
    /// This is readonly for error report. Not move position.
    original: &'a str,

    /// Output
    output_patches: ClipMotionDiffPatch<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
}

impl<'de> Deserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self {
            input,
            original: input,
            output_patches: ClipMotionDiffPatch::DEFAULT,
            current: CurrentState::new(),
        }
    }

    fn parse_next<O>(
        &mut self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O> {
        parser
            .parse_next(&mut self.input)
            .map_err(|err| Error::Context { err })
    }

    /// Parse by argument parser no consume.
    ///
    /// If an error occurs, it is converted to [`ReadableError`] and returned.
    fn parse_peek<O>(
        &self,
        mut parser: impl Parser<&'de str, O, ErrMode<ContextError>>,
    ) -> Result<O> {
        let (_, res) = parser
            .parse_peek(self.input)
            .map_err(|err| Error::Context { err })?;
        Ok(res)
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

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 file patch
    fn root(&mut self) -> Result<()> {
        self.parse_next(multispace0)?;

        while let Some(line_kind) = self.current.next() {
            match line_kind {
                LineKind::ClipId => {
                    let should_take = self.parse_opt_start_comment()?;

                    let clip_id =
                        self.parse_next(one_line.context(Expected(Description("clip_id: Str"))))?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("clip_id = {clip_id:#?}");

                    if should_take {
                        self.current.replace_one(clip_id)?;
                        self.parse_opt_close_comment()?;
                    }
                }
                LineKind::Duration => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.parse_next(multispace0)?;

                    let duration = self.parse_next(
                        verify_line_parses_to::<f32>
                            .context(Expected(Description("duration: f32"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("duration = {duration:#?}");

                    if should_take {
                        self.current.replace_one(duration)?;
                        self.parse_opt_close_comment()?;
                    }
                    self.parse_next(multispace0)?;
                }
                LineKind::TranslationLen | LineKind::RotationLen => {
                    let diff_start = self.parse_opt_start_comment()?;
                    if diff_start {
                        self.current.set_range_start(0)?;
                    }
                    let _len = self.parse_next(
                        verify_line_parses_to::<usize>
                            .context(Expected(Description("length: usize"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("{line_kind:#?} = {_len:#?}");
                }
                LineKind::Translation => {
                    // until rotation length line
                    let mut start_index = 0;
                    while self
                        .parse_peek(opt(verify_line_parses_to::<usize>))?
                        .is_none()
                    {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }

                        if self.parse_next(opt(delete_this_line))?.is_some() {
                            start_index += 1;
                            self.current.increment_translations_range();
                        } else {
                            self.transition()?;
                        }

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;
                        start_index += 1;
                    }
                }
                LineKind::Rotation => {
                    let mut start_index = 0;
                    while self.parse_peek(opt(eof))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }

                        if self.parse_next(opt(delete_this_line))?.is_some() {
                            start_index += 1;
                            self.current.increment_rotations_range();
                        } else {
                            self.rotation()?;
                        }

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;
                        start_index += 1;
                    }
                    break;
                }
            };
        }

        self.parse_next(multispace0)?;
        if !self.input.is_empty() {
            return Err(Error::IncompleteParse);
        }

        Ok(())
    }

    fn transition(&mut self) -> Result<()> {
        let time = self
            .parse_next(from_word_and_space::<f32>.context(Expected(Description("time: f32"))))?;
        let x =
            self.parse_next(from_word_and_space::<f32>.context(Expected(Description("x: f32"))))?;
        let y =
            self.parse_next(from_word_and_space::<f32>.context(Expected(Description("y: f32"))))?;
        let z = {
            let w_parser = till_line_ending
                .verify(|s: &str| s.parse::<f32>().is_ok())
                .context(Expected(Description("z: f32")));
            self.parse_next(w_parser)?
        }
        .into();
        self.parse_next(opt(line_ending))?;

        if self.current.mode_code.is_some() {
            self.current
                .push_as_translation(Translation { time, x, y, z })?;
        }

        Ok(())
    }

    fn rotation(&mut self) -> Result<()> {
        let time = self
            .parse_next(from_word_and_space::<f32>.context(Expected(Description("time: f32"))))?;
        let x =
            self.parse_next(from_word_and_space::<f32>.context(Expected(Description("x: f32"))))?;
        let y =
            self.parse_next(from_word_and_space::<f32>.context(Expected(Description("y: f32"))))?;
        let z =
            self.parse_next(from_word_and_space::<f32>.context(Expected(Description("z: f32"))))?;

        let w = {
            let w_parser = till_line_ending
                .verify(|s: &str| s.parse::<f32>().is_ok())
                .context(Expected(Description("w: f32")));
            self.parse_next(w_parser)?
        }
        .into();
        self.parse_next(opt(line_ending))?;

        let rotation = Rotation { time, x, y, z, w };
        #[cfg(feature = "tracing")]
        tracing::trace!("rotation = {rotation:?}");

        if self.current.mode_code.is_some() {
            self.current.push_as_rotation(rotation)?;
        }
        Ok(())
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Can parse `MOD_CODE ~<mod code>~ OPEN`?
    ///
    /// # Return
    /// Is the mode code comment?
    fn parse_opt_start_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(open_comment))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::ModCode(id) => {
                    self.current.mode_code = Some(id);
                    // When there are no additional differences, it is 100% Remove.
                    let found_end_diff_sym = self.parse_opt_close_comment()?;
                    if found_end_diff_sym {
                        self.current.force_removed = true;
                    };
                    return Ok(true);
                }
                _ => return Ok(false),
            }
        }
        Ok(false)
    }

    /// Processes the close comment (`ORIGINAL` or `CLOSE`) depending on whether it was encountered,
    /// and returns whether it was encountered or not.
    fn parse_opt_close_comment(&mut self) -> Result<bool> {
        if let Some(comment_ty) = self.parse_next(opt(original_or_close_comment))? {
            #[cfg(feature = "tracing")]
            tracing::debug!(?comment_ty);
            match comment_ty {
                CommentKind::Original => {
                    self.current.set_is_passed_original();
                    let op = self.current.judge_operation();
                    if op != Op::Remove {
                        self.parse_next(take_till_close)?;
                        self.merge_to_output()?;
                    }
                    return Ok(true);
                }
                CommentKind::Close => {
                    self.merge_to_output()?;
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// This is the method that is called when a single differential change comment pair finishes calling.
    fn merge_to_output(&mut self) -> Result<(), Error> {
        let op = self.current.judge_operation();
        if let Some(mut partial_patch) = self.current.patch.take() {
            match self.current.current_kind()? {
                LineKind::ClipId => {
                    if let Some(clip_id) = partial_patch.clip_id.take() {
                        self.output_patches.clip_id.replace(clip_id);
                    }
                }
                LineKind::Duration => {
                    if let Some(duration) = partial_patch.duration.take() {
                        self.output_patches.duration.replace(duration);
                    }
                }
                LineKind::TranslationLen | LineKind::RotationLen => {}
                LineKind::Translation => {
                    if let Some(transitions) = partial_patch.translations.take() {
                        let PartialTranslations { range, values } = transitions;
                        let values = if op == Op::Remove { vec![] } else { values };
                        self.output_patches.translations =
                            Some(DiffTransitions { op, range, values });
                    }
                }
                LineKind::Rotation => {
                    if let Some(rotations) = partial_patch.rotations.take() {
                        let PartialRotations { range, values } = rotations;
                        let values = if op == Op::Remove { vec![] } else { values };
                        self.output_patches.rotations = Some(DiffRotations { op, range, values });
                    }
                }
            }

            self.current.clear_flags(); // new patch is generated so clear flags.
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[quick_tracing::init]
    #[test]
    fn test_patch_modes_extended() {
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

        let patches = parse_clip_motion_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            patches,
            ClipMotionDiffPatch {
                clip_id: Some("100".into()),
                duration: None,
                translations: None,
                rotations: Some(DiffRotations {
                    op: Op::Replace,
                    range: 0..3,
                    values: vec![
                        Rotation {
                            time: "6".into(),
                            x: "0".into(),
                            y: "0".into(),
                            z: "0".into(),
                            w: "5".into()
                        },
                        Rotation {
                            time: "6".into(),
                            x: "0".into(),
                            y: "0".into(),
                            z: "0".into(),
                            w: "5".into()
                        },
                        Rotation {
                            time: "6".into(),
                            x: "0".into(),
                            y: "0".into(),
                            z: "0".into(),
                            w: "5".into()
                        },
                    ],
                })
            }
        );
    }

    // #[quick_tracing::init]
    #[test]
    fn test_patch_modes() {
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
1
<!-- MOD_CODE ~test~ OPEN -->
<!-- ORIGINAL -->
2 0 0 0 1
<!-- CLOSE -->
";

        let patches = parse_clip_motion_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipMotionDiffPatch {
            duration: Some("1.25".into()),
            translations: Some(DiffTransitions {
                op: Op::Replace,
                range: 0..3,
                values: vec![
                    Translation {
                        time: "0.43".into(),
                        x: "0".into(),
                        y: "0".into(),
                        z: "0".into(),
                    },
                    Translation {
                        time: "0.50".into(),
                        x: "0".into(),
                        y: "0".into(),
                        z: "0".into(),
                    },
                    Translation {
                        time: "1.32".into(),
                        x: "0".into(),
                        y: "0".into(),
                        z: "0".into(),
                    },
                ],
            }),
            rotations: Some(DiffRotations {
                op: Op::Remove,
                range: 0..1,
                values: vec![],
            }),
            ..Default::default()
        };

        assert_eq!(patches, expected);
    }

    #[test]
    fn test_delete_this_line_patch() {
        let input = "
152
3
10
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
";

        let patches = parse_clip_motion_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            patches,
            ClipMotionDiffPatch {
                clip_id: None,
                duration: None,
                translations: Some(DiffTransitions {
                    op: Op::Replace,
                    range: 0..9,
                    values: vec![
                        Translation {
                            time: "0.0".into(),
                            x: "0".into(),
                            y: "0".into(),
                            z: "0".into()
                        },
                        Translation {
                            time: "0.34".into(),
                            x: "0".into(),
                            y: "72".into(),
                            z: "0".into()
                        },
                        Translation {
                            time: "0.49".into(),
                            x: "0".into(),
                            y: "153".into(),
                            z: "0".into()
                        }
                    ]
                }),
                rotations: None
            }
        );
    }
}
