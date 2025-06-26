#![allow(unused)] // FIXME: Support other formats besides anim_info.
use crate::asdsf::normal::{AnimInfo, Condition};
use crate::asdsf::patch::de::error::Error;
use crate::common_parser::lines::Str;
use json_patch::Op;
use std::{borrow::Cow, ops::Range, slice::Iter};

#[derive(Debug)]
pub struct CurrentState<'input> {
    /// current parsing filed kind
    line_kinds: Iter<'static, ParserKind>,
    current_kind: Option<ParserKind>,
    pub one_field_patch: Option<FieldKind<'input>>,

    /// When present, this signals the start of a differential change
    pub mode_code: Option<&'input str>,

    /// Is passed `<!-- ORIGINAL --!>` xml comment?
    is_passed_original: bool,

    /// If the addition or deletion is `<! -- CLOSE --!>` since it is impossible to determine whether something
    /// is added or deleted until a comment comes in, this is a place to temporarily save them.
    ///
    /// None is nothing diff.
    pub patch: Option<PartialAsdsfPatch<'input>>,

    /// Used only when handling arrays such as `triggers`, `conditions`, `attacks` and `anim_infos`.
    ///
    /// # NOTE
    /// This represents only the continuous partial patches where changes have been made.
    pub main_range: Option<Range<usize>>,

    /// Counts the range of arrays within arrays, such as clip_names in attacks.
    ///
    /// # NOTE
    /// This represents only the continuous partial patches where changes have been made.
    pub sub_range: Option<Range<usize>>,

    pub force_removed: bool,
}

impl<'de> Default for CurrentState<'de> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) enum ParserKind {
    /// - type: `Str`
    #[default]
    Version,

    /// - type: `usize`
    TriggersLen,
    /// - type: `Vec<Str>`
    Triggers,

    /// - type: [`usize`]
    ConditionsLen,
    /// - type: `Vec<Condition>`
    Conditions,

    /// - type: [`usize`]
    AttacksLen,
    /// - type: `Vec<Attack>`
    Attacks,

    /// - type: [`usize`]
    AnimInfosLen,
    /// - type: `Vec<AnimInfo>`
    AnimInfos,
}

#[derive(Debug, Clone)]
pub(super) enum FieldKind<'a> {
    Version(Str<'a>),

    // -- Condition
    /// - type: `Str`
    ConditionVariableName(Str<'a>),
    /// - type: [`i32`]
    ConditionVariableA(Str<'a>),
    /// - type: [`i32`]
    ConditionVariableB(Str<'a>),

    // -- Attack
    /// - type: `Str`
    AttackAttackTrigger(Str<'a>),
    /// - type: [`bool`]
    AttackIsContextual(Str<'a>),
    /// - type: [`usize`]
    AttackClipNamesLen(Str<'a>),
    /// - type: `Vec<Str>`
    AttackClipNames,

    // -- AnimInfo
    /// - type: [`u32`]
    AnimInfoHashedPath(Str<'a>),
    /// - type: [`u32`]
    AnimInfoHashedFileName(Str<'a>),
    /// - type: [`u32`]
    AnimInfoAsciiExtension(Str<'a>),
}

const LINE_KINDS: [ParserKind; 9] = [
    ParserKind::Version,
    ParserKind::TriggersLen,
    ParserKind::Triggers,
    ParserKind::ConditionsLen,
    ParserKind::Conditions,
    ParserKind::AttacksLen,
    ParserKind::Attacks,
    ParserKind::AnimInfosLen,
    ParserKind::AnimInfos,
];

#[derive(Debug, PartialEq, Default)]
pub struct PartialAsdsfPatch<'a> {
    pub version: Option<Cow<'a, str>>,
    pub triggers: Vec<Str<'a>>,

    /// one/seq patch
    pub conditions: Vec<Condition<'a>>,

    // TODO: Consider an intermediate structure for a valid patch for a nested array.
    pub attacks: Option<()>,

    /// one/seq patch
    pub anim_infos: Vec<AnimInfo<'a>>,
}

impl<'de> CurrentState<'de> {
    #[inline]
    pub fn new() -> Self {
        Self {
            line_kinds: LINE_KINDS.iter(),
            current_kind: None,
            one_field_patch: None,
            mode_code: None,
            is_passed_original: false,
            patch: None,
            force_removed: false,
            main_range: None,
            sub_range: None,
        }
    }

    pub(super) fn next(&mut self) -> Option<ParserKind> {
        let next = self.line_kinds.next().copied();
        self.current_kind = next;
        #[cfg(feature = "tracing")]
        tracing::trace!("next = {next:#?}");
        next
    }

    pub(super) fn current_kind(&self) -> Result<ParserKind, Error> {
        self.current_kind.ok_or(Error::EndOfLineKind)
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    #[inline]
    pub fn replace_one(&mut self, value: FieldKind<'de>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        #[cfg(feature = "tracing")]
        tracing::trace!("{self:#?}");
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        self.one_field_patch = Some(value);
        Ok(())
    }

    /// The following is an additional element, so push.
    /// - `<!-- MOD_CODE ~<id>~ --!>` after it is found.
    /// - `<!-- ORIGINAL --!> is not found yet.
    pub fn push_as_trigger(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        if matches!(self.current_kind, Some(ParserKind::Triggers)) {
            self.patch.get_or_insert_default().triggers.push(value);
        } else {
            return Err(Error::ExpectedTrigger);
        };

        Ok(())
    }

    /// Sets the range start index for either transitions or rotations.
    ///
    /// # Errors
    /// - If it is not included in diff (if it does not pass `MOD_CODE`).
    /// - If it is called with a type that is not an array.
    pub const fn set_main_range_start(&mut self, start: usize) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(
                ParserKind::Triggers
                | ParserKind::Conditions
                | ParserKind::Attacks
                | ParserKind::AnimInfos,
            ) => {
                self.main_range = Some(start..start + 1);
            }
            _ => return Err(Error::ExpectedArray),
        }

        Ok(())
    }

    pub const fn increment_main_range(&mut self) {
        if let Some(range_end) = &mut self.main_range {
            range_end.end += 1;
        };
    }

    /// takes range in `Option`
    ///
    /// # Errors
    /// If `Option::is_none`
    pub fn take_main_range(&mut self) -> Result<Range<usize>, Error> {
        self.main_range
            .take()
            .ok_or(Error::NeedMainRangeInformation)
    }

    /// Sets the range start index for either transitions or rotations.
    ///
    /// # Errors
    /// - If it is not included in diff (if it does not pass `MOD_CODE`).
    /// - If it is called with a type that is not an array.
    pub const fn set_sub_range_start(&mut self, start: usize) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(
                ParserKind::Triggers
                | ParserKind::Conditions
                | ParserKind::Attacks
                | ParserKind::AnimInfos,
            ) => {
                self.main_range = Some(start..start + 1);
            }
            _ => return Err(Error::ExpectedArray),
        }

        Ok(())
    }

    pub const fn increment_sub_range(&mut self) {
        if let Some(range_end) = &mut self.sub_range {
            range_end.end += 1;
        };
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
