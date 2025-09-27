mod flags;

use winnow::ascii::{line_ending, space0, space1, Caseless};
use winnow::combinator::{alt, opt, repeat, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::patterns::pair_and_kill::flags::{
    parse_anim_flags, FNISPairAndKillMoveAnimFlagSet,
};

#[derive(Debug, PartialEq)]
pub struct FNISPairedAndKillAnimation<'a> {
    pub kind: FNISPairedType,
    pub flag_set: FNISPairAndKillMoveAnimFlagSet<'a>,
    pub anim_event: &'a str,
    pub anim_file: &'a str,
    /// Animation objects
    pub anim_objects: Vec<AnimObject<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FNISPairedType {
    /// paired animation
    Pa,
    /// kill move
    Km,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimObject<'a> {
    /// e.g. `AnimObjectSword`
    name: &'a str,
    role: ActorRole,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActorRole {
    /// Active actor: initiates the paired animation
    #[default]
    Active,
    /// Passive actor: receives the paired animation
    Passive,
}

pub fn parse_paired_animation<'a>(
    input: &mut &'a str,
) -> ModalResult<FNISPairedAndKillAnimation<'a>> {
    seq!(FNISPairedAndKillAnimation {
        _: space0,
        kind: alt((Caseless("pa").value(FNISPairedType::Pa), Caseless("km").value(FNISPairedType::Km))),
        _: space1,
        flag_set: parse_anim_flags,
        _: space1,

        anim_event: take_till(1.., [' ' , '\t']).context(StrContext::Label("anim_event: str")),
        _: space1,
        anim_file: take_till(1.., [' ' , '\t', '\r', '\n']).context(StrContext::Label("anim_file: str")),
        _: space0,
        anim_objects: repeat(0.., parse_anim_object_numbered),
        _: opt(line_ending),
    })
    .context(StrContext::Label("FNIS Paired/KillMove Animation"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: pa|km [-<option,option ...>] <AnimEvent> <AnimFile> [<AnimObject>/<number> ...]"
    )))
    .parse_next(input)
}

/// `<AnimObject>/<1 or 2>`
fn parse_anim_object_numbered<'a>(input: &mut &'a str) -> ModalResult<AnimObject<'a>> {
    winnow::seq! {
        AnimObject {
            name: take_till(1.., ['/']),
            _: "/",
            role: alt((
                "1".value(ActorRole::Active),
                "2".value(ActorRole::Passive),
            ))
            .context(StrContext::Expected(StrContextValue::Description("1 (Active) | 2 (Passive)"))),
            _: space0,
    }}
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::{
        combinator::{alt_anim::Trigger, flags::FNISAnimFlags},
        test_helpers::must_parse,
    };

    #[test]
    fn test_parse_paired_animation_valid() {
        let parsed = must_parse(
            parse_paired_animation,
            "pa -o,D3.466667,Tnpcsoundplay.npckillchop/2.555,T2_killactor/3.333 HugB paired_hugb.hkx AnimObjectSword/1 AnimObjectAxe/2\n",
        );

        let expected = FNISPairedAndKillAnimation {
            kind: FNISPairedType::Pa,
            flag_set: FNISPairAndKillMoveAnimFlagSet {
                flags: FNISAnimFlags::AnimObjects,
                duration: 3.466667,
                triggers: vec![
                    Trigger {
                        event: "npcsoundplay.npckillchop",
                        time: 2.555,
                    },
                    Trigger {
                        event: "2_killactor",
                        time: 3.333,
                    },
                ],
            },
            anim_event: "HugB",
            anim_file: "paired_hugb.hkx",
            anim_objects: vec![
                AnimObject {
                    name: "AnimObjectSword",
                    role: ActorRole::Active,
                },
                AnimObject {
                    name: "AnimObjectAxe",
                    role: ActorRole::Passive,
                },
            ],
        };

        assert_eq!(parsed, expected);
    }
}
