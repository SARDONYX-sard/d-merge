use super::current_state::CurrentState;
use crate::{
    adsf::patch::de::anim_header::AnimHeaderDiffPatch,
    common_parser::{
        comment::{open_comment, original_or_close_comment, take_till_close, CommentKind},
        lines::{one_line, verify_line_parses_to},
    },
};
use crate::{
    adsf::patch::de::{
        anim_header::{current_state::PartialProjectAssets, DiffProjectAssets, LineKind},
        error::{Error, Result},
    },
    common_parser::{delete_line::delete_this_line, lines::num_bool_line},
};
use json_patch::Op;
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::multispace0,
    combinator::opt,
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    Parser,
};

/// Parse animationdatasinglefile.txt clip motion block patch.
///
/// # Errors
/// Parse failed.
pub fn parse_anim_header_diff_patch(input: &str) -> Result<AnimHeaderDiffPatch<'_>, Error> {
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
    output_patches: AnimHeaderDiffPatch<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
}

impl<'de> Deserializer<'de> {
    fn new(input: &'de str) -> Self {
        Self {
            input,
            original: input,
            output_patches: AnimHeaderDiffPatch::DEFAULT,
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
                LineKind::LeadInt => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.parse_next(multispace0)?;

                    let lead_int = self.parse_next(
                        verify_line_parses_to::<i32>
                            .context(Expected(Description("lead_int: i32"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("lead_int = {lead_int:#?}");

                    if should_take {
                        self.current.replace_lead_int(lead_int)?;
                        self.parse_opt_close_comment()?;
                    }
                    self.parse_next(multispace0)?;
                }
                LineKind::ProjectAssetsLen => {
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
                LineKind::ProjectAssets => {
                    let mut start_index = 0;
                    // until has_motion_data line
                    while self.parse_peek(opt(num_bool_line))?.is_none() {
                        let diff_start = self.parse_opt_start_comment()?;
                        if diff_start {
                            self.current.set_range_start(start_index)?;
                        }

                        if self.parse_next(opt(delete_this_line))?.is_some() {
                            start_index += 1;
                            self.current.increment_project_assets_range();
                        } else {
                            let project_asset = self.parse_next(one_line)?;
                            if self.current.mode_code.is_some() {
                                self.current.push_as_project_assets(project_asset)?;
                            }
                        }

                        self.parse_opt_close_comment()?;
                        self.parse_next(multispace0)?;
                        start_index += 1;
                    }
                }
                LineKind::HasMotionData => {
                    let should_take = self.parse_opt_start_comment()?;
                    self.parse_next(multispace0)?;

                    let _has_motion_data = self.parse_next(
                        num_bool_line.context(Expected(Description("has_motion_data: '0' | '1'"))),
                    )?;
                    #[cfg(feature = "tracing")]
                    tracing::trace!("has_motion_data = {_has_motion_data}");

                    if should_take {
                        self.parse_opt_close_comment()?;
                    }
                    self.parse_next(multispace0)?;
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
                LineKind::LeadInt => {
                    if let Some(lead_int) = partial_patch.lead_int.take() {
                        self.output_patches.lead_int.replace(lead_int);
                    }
                }
                LineKind::ProjectAssetsLen | LineKind::HasMotionData => {}
                LineKind::ProjectAssets => {
                    if let Some(project_assets) = partial_patch.project_assets.take() {
                        let PartialProjectAssets { range, values } = project_assets;
                        let values = if op == Op::Remove { vec![] } else { values };
                        self.output_patches.project_assets =
                            Some(DiffProjectAssets { op, range, values });
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
        // 432: 1                                    // <-- lead_int (maybe only 1)
        // 433: 3                                    // <-- project_assets_len: usize
        // 435: Behaviors\ChickenBehavior.hkx        // <-- project_asset[0]
        // 436: Characters\ChickenCharacter.hkx      // <-- project_asset[1]
        // 437: Character Assets\skeleton.HKX        // <-- project_asset[2]
        // 439: 1                                    // <-- has motion data (1/0): bool
        let input = r##"

        <!-- MOD_CODE ~test~ OPEN -->
93
<!-- ORIGINAL -->
1
<!-- CLOSE -->
3
Behaviors\ChickenBehavior.hkx
<!-- MOD_CODE ~test~ OPEN -->
Characters\ChickenTest0.hkx
Characters\ChickenTest1.hkx
<!-- ORIGINAL -->
Characters\ChickenCharacter.hkx
Character Assets\skeleton.HKX
<!-- CLOSE -->
1

"##;

        let patches = parse_anim_header_diff_patch(input).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            patches,
            AnimHeaderDiffPatch {
                lead_int: Some("93".into()),
                project_assets: Some(DiffProjectAssets {
                    op: Op::Replace,
                    range: 1..3,
                    values: vec![
                        "Characters\\ChickenTest0.hkx".into(),
                        "Characters\\ChickenTest1.hkx".into(),
                    ],
                })
            }
        );
    }
}
