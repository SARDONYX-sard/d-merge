use crate::asdsf::patch::de::{AnimInfosDiff, ConditionsDiff, ParserKind};
use crate::{asdsf::patch::de::error::Error, common_parser::lines::Str};
use json_patch::Op;
use std::{borrow::Cow, ops::Range, slice::Iter};

#[derive(Debug)]
pub struct CurrentState<'input> {
    /// current parsing filed kind
    line_kinds: Iter<'static, ParserKind>,
    current_kind: Option<ParserKind>,

    /// When present, this signals the start of a differential change
    pub mode_code: Option<&'input str>,

    /// Is passed `<!-- ORIGINAL --!>` xml comment?
    is_passed_original: bool,

    /// If the addition or deletion is `<! -- CLOSE --!>` since it is impossible to determine whether something
    /// is added or deleted until a comment comes in, this is a place to temporarily save them.
    ///
    /// None is nothing diff.
    pub patch: Option<PartialAsdsfPatch<'input>>,

    pub force_removed: bool,
}

impl<'de> Default for CurrentState<'de> {
    fn default() -> Self {
        Self::new()
    }
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
    pub triggers: Option<PartialTriggers<'a>>,
    pub conditions: ConditionsDiff<'a>,
    pub attacks: Option<PartialAttacks<'a>>,
    pub anim_infos: AnimInfosDiff<'a>,
}

/// not judge operation yet at this time.
#[derive(Debug, PartialEq, Default)]
pub struct PartialTriggers<'input> {
    pub range: Range<usize>,
    pub values: Vec<Str<'input>>,
}

/// not judge operation yet at this time.
#[derive(Debug, PartialEq, Default)]
pub struct PartialAttacks<'input> {
    pub range: Range<usize>,
    pub values: Vec<Str<'input>>,
}

#[allow(unused)]
enum OnePatchRequest {
    Version,
    ConditionVariableName,
    ConditionValueA,
    ConditionValueB,

    AnimInfoHashedPath,
    AnimInfoHashedFileName,
    AnimInfoAsciiExtension,
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
    pub fn replace_one(&mut self, value: Cow<'de, str>) -> Result<(), Error> {
        let is_in_diff = self.mode_code.is_some();
        #[cfg(feature = "tracing")]
        tracing::trace!("{self:#?}");
        if !is_in_diff {
            return Err(Error::NeedInModDiff);
        }

        match self.current_kind {
            Some(ParserKind::Version) => {
                self.patch.get_or_insert_default().version = Some(value);
            }
            Some(ParserKind::TriggersLen) => {}
            Some(ParserKind::Conditions) => {
                self.patch
                    .get_or_insert_default()
                    .conditions
                    .one
                    .insert(0, Default::default());
            }
            Some(ParserKind::AnimInfos) => {
                self.patch
                    .get_or_insert_default()
                    .anim_infos
                    .one
                    .insert(0, Default::default());
            }
            _ => return Err(Error::ExpectedOne),
        };
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

        match self.current_kind {
            Some(ParserKind::TriggersLen) => {}
            Some(ParserKind::Triggers) => {
                let trigger_names = self
                    .patch
                    .get_or_insert_default()
                    .triggers
                    .get_or_insert_default();

                trigger_names.range.end += 1;
                trigger_names.values.push(value);
            }
            _ => return Err(Error::ExpectedTrigger),
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
            Some(ParserKind::Triggers | ParserKind::Conditions | ParserKind::Attacks) => {
                let triggers = self
                    .patch
                    .get_or_insert_default()
                    .triggers
                    .get_or_insert_default();
                triggers.range.start = start;
                triggers.range.end = start;
            }
            Some(ParserKind::AnimInfos) => {}
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
