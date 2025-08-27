use crate::common_parser::lines::Str;
use crate::diff_line::error::Error;
use json_patch::Op;
use std::{borrow::Cow, ops::Range};

#[derive(Debug)]
pub struct CurrentState<'input> {
    /// When present, this signals the start of a differential change
    pub mode_code: Option<&'input str>,

    /// Is passed `<!-- ORIGINAL --!>` xml comment?
    is_passed_original: bool,

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

    pub force_removed: bool,
}

impl<'de> Default for CurrentState<'de> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de> CurrentState<'de> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            mode_code: None,
            is_passed_original: false,
            patch: None,
            force_removed: false,
            range: None,
        }
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn push_one_line(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
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
    pub const fn set_range_start(&mut self, start: usize) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        self.range = Some(start..start + 1);
        Ok(())
    }

    pub const fn increment_range(&mut self) {
        if let Some(range_end) = &mut self.range {
            range_end.end += 1;
        };
    }

    /// takes range in `Option`
    ///
    /// # Errors
    /// If `Option::is_none`
    pub fn take_range(&mut self) -> Result<Range<usize>, Error> {
        self.range.take().ok_or(Error::NeedRange)
    }

    /// - `<!-- ORIGINAL --!> is found.
    #[inline]
    pub const fn set_is_passed_original(&mut self) {
        self.is_passed_original = true;
    }

    #[inline]
    pub fn judge_operation(&self) -> Op {
        self.mode_code.map_or(Op::Remove, |_| {
            if self.force_removed {
                return Op::Remove;
            }

            if self.is_passed_original {
                if self.patch.is_some() {
                    Op::Replace
                } else {
                    Op::Remove
                }
            } else {
                Op::Add
            }
        })
    }

    #[inline]
    pub const fn clear_flags(&mut self) {
        self.mode_code = None;
        self.is_passed_original = false;
    }
}
