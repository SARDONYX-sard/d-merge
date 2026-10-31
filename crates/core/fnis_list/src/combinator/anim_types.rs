// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.

use winnow::{
    ModalResult, Parser as _,
    error::{StrContext, StrContextValue},
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

    // -----------------------------------------------------------------------------------------------------------------
    /// Although it does not exist in the FNIS PDF, We added it ourself for when `AnimVar` is declared at the top of the List.
    AnimVar,
}

pub(crate) fn parse_anim_type(input: &mut &str) -> ModalResult<FNISAnimType> {
    winnow::token::take_while(1..=3, |c: char| c.is_alphabetic() || c == '+')
        .verify_map(match_caseless)
        .context(StrContext::Label("AnimType"))
        .context(StrContext::Expected(StrContextValue::Description(
            "One of: b, s, so, fu, fuo, +, ofa, o, pa, km, aa, ch",
        )))
        .parse_next(input)
}

// winnow(v1.0.0) alt tuple 8 limit. So we use `match`
const fn match_caseless(s: &str) -> Option<FNISAnimType> {
    Some(if s.eq_ignore_ascii_case("fuo") {
        FNISAnimType::FurnitureOptimized
    } else if s.eq_ignore_ascii_case("ofa") {
        FNISAnimType::OffsetArm
    } else if s.eq_ignore_ascii_case("aa") {
        FNISAnimType::Alternate
    } else if s.eq_ignore_ascii_case("ch") {
        FNISAnimType::Chair
    } else if s.eq_ignore_ascii_case("fu") {
        FNISAnimType::Furniture
    } else if s.eq_ignore_ascii_case("km") {
        FNISAnimType::KillMove
    } else if s.eq_ignore_ascii_case("pa") {
        FNISAnimType::Paired
    } else if s.eq_ignore_ascii_case("so") {
        FNISAnimType::SequencedOptimized
    } else if s.eq_ignore_ascii_case("+") {
        FNISAnimType::SequencedContinued
    } else if s.eq_ignore_ascii_case("b") {
        FNISAnimType::Basic
    } else if s.eq_ignore_ascii_case("o") {
        FNISAnimType::AnimObject
    } else if s.eq_ignore_ascii_case("s") {
        FNISAnimType::Sequenced
    } else {
        return None;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{must_fail, must_parse};

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
