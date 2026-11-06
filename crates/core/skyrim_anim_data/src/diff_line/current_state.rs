use std::{borrow::Cow, ops::Range};

use crate::{
    common_parser::{line_diff::DiffFlags, lines::Str},
    diff_line::error::Error,
};

#[derive(Debug)]
pub(super) struct CurrentState<'input> {
    /// Where the parser is in a block.
    pub flags: DiffFlags<'input>,

    /// If the addition or deletion is `<! -- CLOSE --!>` since it is impossible to determine whether something
    /// is added or deleted until a comment comes in, this is a place to temporarily save them.
    ///
    /// None is nothing diff.
    pub patch: Option<Vec<Str<'input>>>,

    /// Used only when handling arrays such as `triggers`, `conditions`, `attacks` and `anim_infos`.
    ///
    /// # NOTE
    /// This represents only the continuous partial patches where changes have been made.
    pub range: Option<Range<usize>>,
}

impl<'de> Default for CurrentState<'de> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de> CurrentState<'de> {
    pub(super) const fn new() -> Self {
        Self { flags: DiffFlags::new(), patch: None, range: None }
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub(super) fn push_one_line(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        let is_in_diff = self.flags.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        self.patch.get_or_insert_default().push(value);

        Ok(())
    }

    /// Sets the range start index.
    ///
    /// # Errors
    /// - If it is not included in diff (if it does not pass `MOD_CODE`).
    /// - If it is called with a type that is not an array.
    pub(super) const fn set_range_start(&mut self, start: usize) -> Result<(), Error> {
        let is_in_diff = self.flags.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        self.range = Some(start..start + 1);
        Ok(())
    }

    pub(super) const fn increment_range(&mut self) {
        if let Some(range_end) = &mut self.range {
            range_end.end += 1;
        };
    }

    /// takes range in `Option`
    ///
    /// # Errors
    /// If `Option::is_none`
    pub(super) fn take_range(&mut self) -> Result<Range<usize>, Error> {
        self.range.take().ok_or(Error::NeedRange)
    }
}
