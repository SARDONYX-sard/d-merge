//! - FNIS Animation: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]

use winnow::ascii::{line_ending, space0, space1};
use winnow::combinator::{opt, repeat, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::anim_types::{
    parse_anim_type, FNISAnimType,
};
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::{
    parse_anim_flags, FNISAnimFlagSet,
};

#[derive(Debug, PartialEq)]
pub struct FNISAnimation<'a> {
    pub anim_type: FNISAnimType,
    pub flags: FNISAnimFlagSet<'a>,
    pub anim_event: &'a str,
    pub anim_file: &'a str,
    pub anim_objects: Vec<&'a str>,
}

pub fn parse_fnis_animation<'a>(input: &mut &'a str) -> ModalResult<FNISAnimation<'a>> {
    seq!(FNISAnimation {
            _: space0,
            anim_type: parse_anim_type,
            _: space1,
            _: "-", // <- flags start
            flags: parse_anim_flags,
            _: space1,
            anim_event: take_till(1.., [' ' , '\t']).context(StrContext::Label("anim_event: str")),
            _: space1,
            anim_file: take_till(1.., [' ' , '\t']).context(StrContext::Label("anim_file: str")),
            _: space0,
            anim_objects: repeat(0.., take_till(1.., [' ' , '\t', '\r', '\n']).context(StrContext::Label("anim_objects: Vec<str>"))),
            _: opt(line_ending), // At the end of the file, there is no \n.
    })
    .context(StrContext::Label("FNIS Animation"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::{
        combinator::flags::FNISAnimFlags,
        test_helpers::{must_fail, must_parse},
    };

    #[test]
    fn test_parse_fnis_animation_valid() {
        let parsed = must_parse(
            parse_fnis_animation,
            "+ -o,k MyCheerSA3 MyCheerAnim2.hkx AnimObjectIronSword\n",
        );
        assert_eq!(
            parsed,
            FNISAnimation {
                anim_type: FNISAnimType::SequencedContinued,
                flags: FNISAnimFlagSet {
                    flags: FNISAnimFlags::AnimObjects | FNISAnimFlags::Known,
                    params: vec![]
                },
                anim_event: "MyCheerSA3",
                anim_file: "MyCheerAnim2.hkx",
                anim_objects: vec!["AnimObjectIronSword"]
            }
        );
    }

    #[test]
    fn test_parse_fnis_animation_invalid() {
        must_fail(
            parse_fnis_animation,
            "T s -a,k MyCheerSA1 ..\\idlewave.hkx \n",
        );
    }
}
