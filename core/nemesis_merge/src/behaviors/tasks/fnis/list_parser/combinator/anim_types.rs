// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use winnow::{
    combinator::alt,
    error::{StrContext, StrContextValue},
    ModalResult, Parser as _,
};

/// Core FNIS animation types from `<AnimType>` syntax.
///
/// **based on and quoted from** _Fore's_ **"FNIS for Modders_V6.2.pdf"(© Fore)**,
/// which is part of the FNIS (Fores New Idles in Skyrim) modding documentation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FNISAnimType {
    /// **b** – Basic: simple idle animation with one animation file.
    Basic,
    /// **s** – Sequenced Animation (SA): first of at least 2 animations played as a sequence.
    Sequenced,
    /// **so** – Sequenced Optimized: SA with AnimObjects and optimized Equip/UnEquip.
    SequencedOptimized,
    /// **fu** – Furniture Animation: first of at least 3 animations played on a furniture object.
    Furniture,
    /// **fuo** – Furniture Animation Optimized: fu with AnimObjects and optimized Equip/UnEquip.
    FurnitureOptimized,
    /// **+** – Second-to-last animation of a s/so/fu/fuo or ch definition.
    SequencedContinued,

    /// o - AnimObject: basic animation with one or more AnimObjects
    AnimObject,

    /// **ofa** – Offset Arm Animation: modifies arm position while other animations play.
    OffsetArm,
    /// **pa** – Paired Animation: contains animation data for two actors in one animation file.
    Paired,
    /// **km** – Killmove: paired animation used for the final blow in combat.
    KillMove,
    /// **aa** – Alternate Animation.
    Alternate,
    /// **ch** – Chair Animation.
    Chair,
}

impl FNISAnimType {
    /// To FNIS options.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Basic => "b",
            Self::Sequenced => "s",
            Self::SequencedOptimized => "so",
            Self::Furniture => "fu",
            Self::FurnitureOptimized => "fuo",
            Self::SequencedContinued => "+",
            Self::OffsetArm => "ofa",
            Self::AnimObject => "o",
            Self::Paired => "pa",
            Self::KillMove => "km",
            Self::Alternate => "aa",
            Self::Chair => "ch",
        }
    }
}

pub fn parse_anim_type(input: &mut &str) -> ModalResult<FNISAnimType> {
    alt((
        "fuo".value(FNISAnimType::FurnitureOptimized),
        "ofa".value(FNISAnimType::OffsetArm),
        // 2 char
        "aa".value(FNISAnimType::Alternate),
        "ch".value(FNISAnimType::Chair),
        "fu".value(FNISAnimType::Furniture),
        "km".value(FNISAnimType::KillMove),
        "pa".value(FNISAnimType::Paired),
        "so".value(FNISAnimType::SequencedOptimized),
        // 1 char
        "+".value(FNISAnimType::SequencedContinued),
        "b".value(FNISAnimType::Basic),
        "o".value(FNISAnimType::AnimObject),
        "s".value(FNISAnimType::Sequenced),
    ))
    .context(StrContext::Label("AnimType"))
    .context(StrContext::Expected(StrContextValue::Description(
        "One of: b, s, so, fu, fuo, +, ofa, o, pa, km, aa, ch",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn test_parse_anim_type_valid() {
        assert_eq!(must_parse(parse_anim_type, "b"), FNISAnimType::Basic);
        assert_eq!(must_parse(parse_anim_type, "fu"), FNISAnimType::Furniture);
        assert_eq!(must_parse(parse_anim_type, "pa"), FNISAnimType::Paired);
    }

    #[test]
    fn test_parse_anim_type_invalid() {
        must_fail(parse_anim_type, "xxx");
    }
}
