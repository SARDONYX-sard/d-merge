//! Reading a patch of plain lines, where the marker comments say what happens to the lines between
//! them. What each parser does with the lines is its own.
//!
//! ```text
//! a
//! <!-- MOD_CODE ~m~ OPEN -->    open: [`DiffFlags::mode_code`]
//! b                             the parser keeps the lines of its block
//! <!-- ORIGINAL -->             [`DiffFlags::is_passed_original`], the template's lines are skipped
//! c
//! <!-- CLOSE -->                [`LineDiffParser::merge_to_output`]
//! ```

use json_patch::Op;
use nemesis_patch_syntax::{
    comment::{CommentKind, open_comment, original_or_close_comment, take_till_close},
    cursor::{Cursor, ParseFailure},
};
use winnow::combinator::opt;

/// Where a parser is in a block.
#[derive(Debug, Default)]
pub(crate) struct DiffFlags<'input> {
    /// When present, this signals the start of a differential change.
    pub(crate) mode_code: Option<&'input str>,
    /// Is passed `<!-- ORIGINAL -->`?
    pub(crate) is_passed_original: bool,
    /// Nothing came between `OPEN` and what ends it: the block removes.
    pub(crate) force_removed: bool,
}

impl DiffFlags<'_> {
    pub(crate) const fn new() -> Self {
        Self { mode_code: None, is_passed_original: false, force_removed: false }
    }

    /// What the block does. `has_patch`: it had lines of its own.
    pub(crate) const fn judge_operation(&self, has_patch: bool) -> Op {
        if self.mode_code.is_none() || self.force_removed {
            return Op::Remove;
        }
        match (self.is_passed_original, has_patch) {
            (true, true) => Op::Replace,
            (true, false) => Op::Remove,
            (false, _) => Op::Add,
        }
    }

    /// A block ended. `force_removed` stays, as the parsers have always left it.
    pub(crate) const fn clear(&mut self) {
        self.mode_code = None;
        self.is_passed_original = false;
    }
}

/// A parser of such a patch.
pub(crate) trait LineDiffParser<'de> {
    type Error: ParseFailure;

    fn cursor(&mut self) -> &mut Cursor<'de, Self::Error>;

    fn flags(&mut self) -> &mut DiffFlags<'de>;

    /// Did the block have lines of its own so far?
    fn has_patch(&self) -> bool;

    /// A block ended: what it did goes to the output.
    ///
    /// # Errors
    /// What the parser found does not make a change.
    fn merge_to_output(&mut self) -> Result<(), Self::Error>;

    /// `<!-- MOD_CODE ~id~ OPEN -->`, if it is next.
    ///
    /// # Errors
    /// Parse failed.
    fn parse_opt_start_comment(&mut self) -> Result<bool, Self::Error> {
        let Some(CommentKind::ModCode(id)) = self.cursor().parse_next(opt(open_comment))? else {
            return Ok(false);
        };
        self.flags().mode_code = Some(id);
        // When there are no additional differences, it is 100% Remove.
        if self.parse_opt_close_comment()? {
            self.flags().force_removed = true;
        }
        Ok(true)
    }

    /// `<!-- ORIGINAL -->` or `<!-- CLOSE -->`, if one is next.
    ///
    /// # Errors
    /// Parse failed.
    fn parse_opt_close_comment(&mut self) -> Result<bool, Self::Error> {
        match self.cursor().parse_next(opt(original_or_close_comment))? {
            Some(CommentKind::Original) => {
                self.flags().is_passed_original = true;
                let has_patch = self.has_patch();
                if self.flags().judge_operation(has_patch) != Op::Remove {
                    self.cursor().parse_next(take_till_close)?;
                    self.merge_to_output()?;
                }
                Ok(true)
            }
            Some(CommentKind::Close) => {
                self.merge_to_output()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flags_tell_the_operation() {
        let mut flags = DiffFlags::new();
        assert_eq!(flags.judge_operation(true), Op::Remove, "out of a block");

        flags.mode_code = Some("m");
        assert_eq!(flags.judge_operation(true), Op::Add);
        assert_eq!(flags.judge_operation(false), Op::Add);

        flags.is_passed_original = true;
        assert_eq!(flags.judge_operation(true), Op::Replace);
        assert_eq!(flags.judge_operation(false), Op::Remove);

        flags.force_removed = true;
        assert_eq!(flags.judge_operation(true), Op::Remove);

        flags.clear();
        assert_eq!((flags.mode_code, flags.is_passed_original, flags.force_removed), (None, false, true));
    }
}
