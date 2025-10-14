use crate::common_parser::comment::{close_comment, comment_kind, take_till_close, CommentKind};
use crate::common_parser::lines::one_line;
use crate::diff_line::current_state::CurrentState;
use crate::diff_line::error::{Error, Result};
use crate::diff_line::DiffLines;
use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::multispace0,
    combinator::{eof, opt},
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    Parser,
};

/// Parse multi line diff patch.
///
/// # Errors
/// Parse failed.
pub fn parse_lines_diff_patch(input: &str, priority: usize) -> Result<DiffLines<'_>, Error> {
    let mut deserializer = Deserializer::new(input, priority);
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
    output_patches: DiffLines<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
    priority: usize,
}

impl<'de> Deserializer<'de> {
    const fn new(input: &'de str, priority: usize) -> Self {
        Self {
            input,
            original: input,
            output_patches: DiffLines::DEFAULT,
            current: CurrentState::new(),
            priority,
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
        let mut start_index = 0;
        while self.parse_peek(opt(eof))?.is_none() {
            let diff_start = self.parse_opt_start_comment()?;
            if diff_start {
                self.current.set_range_start(start_index)?;
            }
            let one_line =
                self.parse_next(one_line.context(Expected(Description("one line: Str"))))?;
            if self.current.mode_code.is_some() {
                self.current.push_one_line(one_line)?;
            }

            self.parse_opt_close_comment()?;
            self.parse_next(multispace0)?;
            start_index += 1;
            self.current.increment_range();
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
        if let Some(comment_ty) = self.parse_next(opt(comment_kind))? {
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
        if let Some(comment_ty) = self.parse_next(opt(close_comment))? {
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
            let lines = core::mem::take(&mut partial_patch);
            let values = if op == Op::Remove { vec![] } else { lines };
            let values = ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op,
                        range: self.current.take_range()?,
                    },
                    value: values.into(),
                },
                priority: self.priority,
            };
            self.output_patches.0.push(values);

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
TextReplace.txt
<!-- ORIGINAL -->
Text0.txt
<!-- CLOSE -->
Text1.txt
Text2.txt
Text3.txt
Text4.txt
Text5.txt
Text6.txt
<!-- MOD_CODE ~test~ OPEN -->
TextAdd.txt
TextAdd.txt
<!-- CLOSE -->
";

        let patches = parse_lines_diff_patch(input, 0).unwrap_or_else(|e| panic!("{e}"));
        let expected = DiffLines(vec![
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op: Op::Replace,
                        range: 0..1,
                    },
                    value: simd_json::json_typed! { borrowed, [
                        "TextReplace.txt"
                    ] },
                },
                priority: 0,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq {
                        op: Op::Add,
                        range: 7..9,
                    },
                    value: simd_json::json_typed! { borrowed, [
                        "TextAdd.txt",
                        "TextAdd.txt"
                    ] },
                },
                priority: 0,
            },
        ]);

        assert_eq!(patches, expected);
    }
}
