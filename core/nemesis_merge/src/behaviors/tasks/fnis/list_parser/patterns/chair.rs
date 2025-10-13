//! - FNIS Animation: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]

use winnow::ascii::{space0, space1};
use winnow::combinator::{repeat, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::anim_types::FNISAnimType;
use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;
use crate::behaviors::tasks::fnis::list_parser::combinator::fnis_animation::parse_fnis_animation;
use crate::behaviors::tasks::fnis::list_parser::combinator::{
    flags::FNISAnimFlags, fnis_animation::FNISAnimation,
};

#[derive(Debug, PartialEq)]
pub struct FNISChairAnimation<'a> {
    /// start chair animation
    pub start: FNISAnimation<'a>,
    /// files
    ///
    /// base, var1, var2
    pub sequenced: Vec<&'a str>,
}

pub fn parse_fnis_chair_animation<'a>(input: &mut &'a str) -> ModalResult<FNISChairAnimation<'a>> {
    seq!(FNISChairAnimation{
            _: space0,
            start: parse_fnis_animation.verify(|anim| {
                let is_chair = anim.anim_type == FNISAnimType::Chair;
                let flags = anim.flag_set.flags;
                let has_none_or_anim_obj_only = flags == FNISAnimFlags::AnimObjects || flags.is_empty();
                is_chair && has_none_or_anim_obj_only
            }).context(StrContext::Label("Chair start animation: only -o or no options are allowed")),
            sequenced: parse_sequenced_animation,
    })
    .context(StrContext::Label("FNIS Chair Animation"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: ch [-o] <AnimEvent_1> <AnimFile1> [<AnimObject> …]
        + <Unused_AnimEvent_dummy2> <AnimFile2>
        + <Unused_AnimEvent_dummy3> <AnimFile3>
        + <Unused_AnimEvent_dummy4> <AnimFile4>",
    )))
    .parse_next(input)
}

fn parse_sequenced_animation<'a>(input: &mut &'a str) -> ModalResult<Vec<&'a str>> {
    repeat(3.., parse_file)
        .context(StrContext::Expected(StrContextValue::Description(
            "Chair animation requires at least 4 consecutive animations.",
        )))
        .parse_next(input)
}

fn parse_file<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let (file,) = seq! {
        _: space0,
        _: "+".context(StrContext::Expected(StrContextValue::StringLiteral("+"))),
        _: space1,
        _: take_till(1.., [' ' , '\t']).context(StrContext::Label("dummy_event: str")),
        _: space1,
        take_till(1.., [' ' , '\t', '\r', '\n']).context(StrContext::Label("anim_file: str")),
        _: skip_ws_and_comments,
    }
    .parse_next(input)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::{
        combinator::{anim_types::FNISAnimType, flags::FNISAnimFlagSet},
        test_helpers::must_parse,
    };

    #[test]
    fn test_parse_fnis_animation_valid() {
        let parsed = must_parse(
            parse_fnis_chair_animation,
            r"ch -o PlayFluteSitting PlayFluteSittingStart.hkx AnimObjectFlute
+ PlayFluteSitting_2 PlayFluteSittingIdlebase.hkx
+ PlayFluteSitting_3 PlayFluteSittingIdlevar1.hkx
+ PlayFluteSitting_4 PlayFluteSittingIdlevar2.hkx",
        );

        assert_eq!(
            parsed,
            FNISChairAnimation {
                start: FNISAnimation {
                    anim_type: FNISAnimType::Chair,
                    anim_event: "PlayFluteSitting",
                    anim_file: "PlayFluteSittingStart.hkx",
                    anim_objects: vec!["AnimObjectFlute"],
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::AnimObjects,
                        ..Default::default()
                    },
                    anim_vars: vec![],
                    motions: vec![],
                    rotations: vec![],
                },
                sequenced: vec![
                    "PlayFluteSittingIdlebase.hkx",
                    "PlayFluteSittingIdlevar1.hkx",
                    "PlayFluteSittingIdlevar2.hkx"
                ]
            }
        );
    }
}
