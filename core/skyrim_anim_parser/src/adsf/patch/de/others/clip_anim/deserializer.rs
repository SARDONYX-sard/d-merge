use crate::adsf::patch::de::error::{Error, Result};
use crate::adsf::patch::de::others::clip_anim::{
    current_state::{CurrentState, PartialRotations},
    ClipAnimDiffPatch, DiffTriggerNames, LineKind,
};
use crate::common_parser::comment::{
    open_comment, original_or_close_comment, take_till_close, CommentKind,
};
use crate::common_parser::delete_line::delete_this_line;
use crate::common_parser::lines::{one_line, verify_line_parses_to};
use json_patch::Op;
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::multispace0,
    combinator::{eof, opt},
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    Parser,
};

/// Parse animationdatasinglefile.txt clip animation block patch.
///
/// # Errors
/// Parse failed.
pub fn parse_clip_anim_diff_patch(input: &str) -> Result<ClipAnimDiffPatch<'_>, Error> {
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
    output_patches: ClipAnimDiffPatch<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
}

impl<'de> Deserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self {
            input,
            original: input,
            output_patches: ClipAnimDiffPatch::DEFAULT,
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
                LineKind::Name => {
                    let should_take = self.parse_opt_start_comment()?;

                    let name =
                        self.parse_next(one_line.context(Expected(Description("name: Str"))))?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("name = {name:#?}");

                    if should_take {
                        self.current.replace_one(name)?;
                        self.parse_opt_close_comment()?;
                    }
                }
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
                LineKind::PlayBackSpeed => {
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
                LineKind::CropStartLocalTime => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.parse_next(multispace0)?;

                    let crop_start_local_time = self.parse_next(
                        verify_line_parses_to::<f32>
                            .context(Expected(Description("crop_start_local_time: f32"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("crop_start_local_time = {crop_start_local_time:#?}");

                    if should_take {
                        self.current.replace_one(crop_start_local_time)?;
                        self.parse_opt_close_comment()?;
                    }
                    self.parse_next(multispace0)?;
                }
                LineKind::CropEndLocalTime => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.parse_next(multispace0)?;

                    let crop_end_local_time = self.parse_next(
                        verify_line_parses_to::<f32>
                            .context(Expected(Description("crop_end_local_time: f32"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("crop_end_local_time = {crop_end_local_time:#?}");

                    if should_take {
                        self.current.replace_one(crop_end_local_time)?;
                        self.parse_opt_close_comment()?;
                    }
                    self.parse_next(multispace0)?;
                }
                LineKind::TriggerNamesLen => {
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
                LineKind::TriggerNames => {
                    let mut start_index = 0;
                    while self.parse_peek(opt(eof))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }

                        if self.parse_next(opt(delete_this_line))?.is_some() {
                            start_index += 1;
                            self.current.increment_trigger_names_range();
                        } else {
                            let trigger_name = self.parse_next(
                                one_line.context(Expected(Description("trigger_name: Str"))),
                            )?;
                            if self.current.mode_code.is_some() {
                                self.current.push_as_trigger_name(trigger_name)?;
                            }
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
                LineKind::Name => {
                    if let Some(name) = partial_patch.name.take() {
                        self.output_patches.name.replace(name);
                    }
                }
                LineKind::ClipId => {
                    if let Some(clip_id) = partial_patch.clip_id.take() {
                        self.output_patches.clip_id.replace(clip_id);
                    }
                }
                LineKind::PlayBackSpeed => {
                    if let Some(duration) = partial_patch.play_back_speed.take() {
                        self.output_patches.play_back_speed.replace(duration);
                    }
                }
                LineKind::CropStartLocalTime => {
                    if let Some(crop_start_time) = partial_patch.crop_start_local_time.take() {
                        self.output_patches
                            .crop_start_local_time
                            .replace(crop_start_time);
                    }
                }
                LineKind::CropEndLocalTime => {
                    if let Some(crop_end_local_time) = partial_patch.crop_end_local_time.take() {
                        self.output_patches
                            .crop_end_local_time
                            .replace(crop_end_local_time);
                    }
                }
                LineKind::TriggerNamesLen => {}
                LineKind::TriggerNames => {
                    if let Some(trigger_names) = partial_patch.trigger_names.take() {
                        let PartialRotations { range, values } = trigger_names;
                        let values = if op == Op::Remove { vec![] } else { values };
                        self.output_patches.trigger_names =
                            Some(DiffTriggerNames { op, range, values });
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
    fn test_replace_anim_block_diff_patch() {
        let input = "
<!-- MOD_CODE ~test~ OPEN -->
TurnRight[test]
<!-- ORIGINAL -->
TurnRight[mirrored]
<!-- CLOSE -->
18
1
0
2
clipStart:6.65767
<!-- MOD_CODE ~test~ OPEN -->
clipEnd:8.1234
<!-- ORIGINAL -->
clipEnd:6.65767
<!-- CLOSE -->
";

        let patches = parse_clip_anim_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipAnimDiffPatch {
            name: Some("TurnRight[test]".into()),
            trigger_names: Some(DiffTriggerNames {
                op: Op::Replace,
                range: 1..2,
                values: vec!["clipEnd:8.1234".into()],
            }),
            ..Default::default()
        };

        assert_eq!(patches, expected);
    }

    // #[quick_tracing::init]
    #[test]
    fn test_add_anim_block_diff_patch() {
        let input = "
<!-- MOD_CODE ~test~ OPEN -->
TurnRight[test]
<!-- ORIGINAL -->
TurnRight[mirrored]
<!-- CLOSE -->
18
1
0
0
<!-- MOD_CODE ~test~ OPEN -->
clipStart:6.65767
clipEnd:6.65767
<!-- CLOSE -->
";

        let patches = parse_clip_anim_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        let expected = ClipAnimDiffPatch {
            name: Some("TurnRight[test]".into()),
            trigger_names: Some(DiffTriggerNames {
                op: Op::Add,
                range: 0..2,
                values: vec!["clipStart:6.65767".into(), "clipEnd:6.65767".into()],
            }),
            ..Default::default()
        };

        assert_eq!(patches, expected);
    }
}
