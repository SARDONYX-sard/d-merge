use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
use nemesis_patch_syntax::cursor::Cursor;
use winnow::{
    Parser,
    ascii::multispace0,
    combinator::{eof, opt},
    error::{StrContext::*, StrContextValue::*},
};

use crate::{
    common_parser::{
        line_diff::{DiffFlags, LineDiffParser},
        lines::one_line,
    },
    diff_line::{
        DiffLines,
        current_state::CurrentState,
        error::{Error, Result},
    },
};

/// Parse multi line diff patch.
///
/// # Errors
/// Parse failed.
pub fn parse_lines_diff_patch(input: &str, priority: usize) -> Result<DiffLines<'_>, Error> {
    let mut deserializer = Deserializer::new(input, priority);
    deserializer.root().map_err(|err| deserializer.cursor.to_readable(err))?;
    Ok(deserializer.output_patches)
}

/// Nemesis patch deserializer
#[derive(Debug)]
struct Deserializer<'a> {
    /// Where the parsing is.
    cursor: Cursor<'a, Error>,

    /// Output
    output_patches: DiffLines<'a>,

    /// - `<! -- CLOSE --! >`(XML) where it is temporarily stored because the operation type is unknown until a comment is found.
    /// - `<! -- CLOSE --! >` is found, have it added to `output_patches`.
    pub current: CurrentState<'a>,
    priority: usize,
}

impl<'de> Deserializer<'de> {
    const fn new(input: &'de str, priority: usize) -> Self {
        Self { cursor: Cursor::new(input), output_patches: DiffLines::DEFAULT, current: CurrentState::new(), priority }
    }

    // /////////////////////////////////////////////////////////////////////////////////////////////////////////////////

    /// Parse 1 file patch
    fn root(&mut self) -> Result<()> {
        self.cursor.parse_next(multispace0)?;
        let mut start_index = 0;
        while self.cursor.parse_peek(opt(eof))?.is_none() {
            let diff_start = self.parse_opt_start_comment()?;
            if diff_start {
                self.current.set_range_start(start_index)?;
            }
            let one_line = self.cursor.parse_next(one_line.context(Expected(Description("one line: Str"))))?;
            if self.current.flags.mode_code.is_some() {
                self.current.push_one_line(one_line)?;
            }

            self.parse_opt_close_comment()?;
            self.cursor.parse_next(multispace0)?;
            start_index += 1;
            self.current.increment_range();
        }

        self.cursor.parse_next(multispace0)?;
        if !self.cursor.input.is_empty() {
            return Err(Error::IncompleteParse);
        }

        Ok(())
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
}

impl<'de> LineDiffParser<'de> for Deserializer<'de> {
    type Error = Error;

    fn cursor(&mut self) -> &mut Cursor<'de, Error> {
        &mut self.cursor
    }

    fn flags(&mut self) -> &mut DiffFlags<'de> {
        &mut self.current.flags
    }

    fn has_patch(&self) -> bool {
        self.current.patch.is_some()
    }

    /// This is the method that is called when a single differential change comment pair finishes calling.
    fn merge_to_output(&mut self) -> Result<(), Error> {
        let op = self.current.flags.judge_operation(self.current.patch.is_some());
        if let Some(mut partial_patch) = self.current.patch.take() {
            let lines = core::mem::take(&mut partial_patch);
            let values = if op == Op::Remove { vec![] } else { lines };
            let values = ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq { op, range: self.current.take_range()? },
                    value: values.into(),
                },
                priority: self.priority,
            };
            self.output_patches.0.push(values);

            self.current.flags.clear(); // new patch is generated so clear flags.
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
                    action: Action::Seq { op: Op::Replace, range: 0..1 },
                    value: simd_json::json_typed! { borrowed, [
                        "TextReplace.txt"
                    ] },
                },
                priority: 0,
            },
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Seq { op: Op::Add, range: 7..9 },
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
