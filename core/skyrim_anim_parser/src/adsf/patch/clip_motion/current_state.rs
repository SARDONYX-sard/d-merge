use crate::adsf::{
    patch::{clip_motion::LineKind, error::Error},
    Rotation, Translation,
};
use json_patch::Op;
use std::{borrow::Cow, ops::Range, slice::Iter};

#[derive(Debug)]
pub struct CurrentState<'input> {
    /// current parsing filed kind
    line_kinds: Iter<'static, LineKind>,
    current_kind: Option<LineKind>,

    /// When present, this signals the start of a differential change
    pub mode_code: Option<&'input str>,

    /// Is passed `<!-- ORIGINAL --!>` xml comment?
    is_passed_original: bool,

    /// If the addition or deletion is `<! -- CLOSE --!>` since it is impossible to determine whether something
    /// is added or deleted until a comment comes in, this is a place to temporarily save them.
    ///
    /// None is nothing diff.
    pub patch: Option<PartialAdsfPatch<'input>>,

    pub force_removed: bool,
}

const LINE_KINDS: [LineKind; 6] = [
    LineKind::ClipId,
    LineKind::Duration,
    LineKind::TranslationLen,
    LineKind::Translation,
    LineKind::RotationLen,
    LineKind::Rotation,
];

#[derive(Debug, PartialEq, Default)]
pub struct PartialAdsfPatch<'a> {
    pub clip_id: Option<Cow<'a, str>>,
    pub duration: Option<Cow<'a, str>>,
    pub translations: Option<PartialTranslations<'a>>,
    pub rotations: Option<PartialRotations<'a>>,
}

/// not judge operation yet at this time.
#[derive(Debug, PartialEq, Default)]
pub struct PartialTranslations<'input> {
    pub range: Range<usize>,
    pub values: Vec<Translation<'input>>,
}

/// not judge operation yet at this time.
#[derive(Debug, PartialEq, Default)]
pub struct PartialRotations<'input> {
    pub range: Range<usize>,
    pub values: Vec<Rotation<'input>>,
}

impl<'de> CurrentState<'de> {
    #[inline]
    pub fn new() -> Self {
        Self {
            line_kinds: LINE_KINDS.iter(),
            current_kind: None,
            mode_code: None,
            is_passed_original: false,
            patch: None,
            force_removed: false,
        }
    }

    pub(super) fn next(&mut self) -> Option<LineKind> {
        let next = self.line_kinds.next().copied();
        self.current_kind = next;
        #[cfg(feature = "tracing")]
        tracing::trace!("next = {next:#?}");
        next
    }

    pub(super) fn current_kind(&self) -> Result<LineKind, Error> {
        self.current_kind.ok_or(Error::EndOfLineKind)
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn replace_one(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        #[cfg(feature = "tracing")]
        tracing::trace!("{self:#?}");
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::ClipId) => self.patch.get_or_insert_default().clip_id = Some(value),
            Some(LineKind::Duration) => self.patch.get_or_insert_default().duration = Some(value),
            Some(LineKind::TranslationLen | LineKind::RotationLen) => {}
            _ => return Err(Error::ExpectedOne),
        };
        Ok(())
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn push_as_translation(&mut self, value: Translation<'de>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        #[cfg(feature = "tracing")]
        tracing::trace!("{self:#?}");
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::TranslationLen | LineKind::RotationLen) => {}
            Some(LineKind::Translation) => {
                let transition = self
                    .patch
                    .get_or_insert_default()
                    .translations
                    .get_or_insert_default();

                transition.range.end += 1;
                transition.values.push(value);
            }
            _ => return Err(Error::ExpectedTransition),
        };

        Ok(())
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn push_as_rotation(&mut self, value: Rotation<'de>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::TranslationLen | LineKind::RotationLen) => {}
            Some(LineKind::Rotation) => {
                let rotations = self
                    .patch
                    .get_or_insert_default()
                    .rotations
                    .get_or_insert_default();

                rotations.range.end += 1;
                rotations.values.push(value);
            }
            _ => return Err(Error::ExpectedTransition),
        };

        Ok(())
    }

    /// Sets the range start index for either transitions or rotations.
    pub fn set_range_start(&mut self, start: usize) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::Translation) => {
                let transitions = self
                    .patch
                    .get_or_insert_default()
                    .translations
                    .get_or_insert_default();
                transitions.range.start = start;
                transitions.range.end = start;
            }
            Some(LineKind::Rotation) => {
                let rotations = self
                    .patch
                    .get_or_insert_default()
                    .rotations
                    .get_or_insert_default();
                rotations.range.start = start;
                rotations.range.end = start;
            }
            _ => return Err(Error::ExpectedArray),
        }

        Ok(())
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
