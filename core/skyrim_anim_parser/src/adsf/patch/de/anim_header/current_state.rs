use crate::adsf::patch::de::anim_header::LineKind;
use crate::adsf::patch::de::error::Error;
use json_patch::Op;
use std::{borrow::Cow, slice::Iter};

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

const LINE_KINDS: [LineKind; 4] = [
    LineKind::LeadInt,
    LineKind::ProjectAssetsLen,
    LineKind::ProjectAssets,
    LineKind::HasMotionData,
];

#[derive(Debug, PartialEq, Default)]
pub struct PartialAdsfPatch<'a> {
    pub lead_int: Option<Cow<'a, str>>,
    pub project_assets: Option<PartialProjectAssets<'a>>,
}

/// not judge operation yet at this time.
#[derive(Debug, PartialEq, Default)]
pub struct PartialProjectAssets<'input> {
    pub range: core::ops::Range<usize>,
    pub values: Vec<Cow<'input, str>>,
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
    pub fn replace_lead_int(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        if self.mode_code.is_none() {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::LeadInt) => self.patch.get_or_insert_default().lead_int = Some(value),
            Some(LineKind::HasMotionData | LineKind::ProjectAssetsLen) => {}
            Some(LineKind::ProjectAssets) | None => return Err(Error::ExpectedOne),
        };
        Ok(())
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn push_as_project_assets(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        #[cfg(feature = "tracing")]
        tracing::trace!("{self:#?}");
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::HasMotionData | LineKind::ProjectAssetsLen) => {}
            Some(LineKind::ProjectAssets) => {
                let project_assets = self
                    .patch
                    .get_or_insert_default()
                    .project_assets
                    .get_or_insert_default();

                project_assets.range.end += 1;
                project_assets.values.push(value);
            }
            Some(LineKind::LeadInt) => return Err(Error::ExpectedArray),
            None => return Err(Error::IncompleteParse),
        };

        Ok(())
    }

    pub fn increment_project_assets_range(&mut self) {
        let project_assets = self
            .patch
            .get_or_insert_default()
            .project_assets
            .get_or_insert_default();

        project_assets.range.end += 1;
    }

    /// Sets the range start index for either transitions or rotations.
    pub fn set_range_start(&mut self, start: usize) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(LineKind::ProjectAssetsLen | LineKind::ProjectAssets) => {
                let project_assets = self
                    .patch
                    .get_or_insert_default()
                    .project_assets
                    .get_or_insert_default();
                project_assets.range.start = start;
                project_assets.range.end = start;
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
