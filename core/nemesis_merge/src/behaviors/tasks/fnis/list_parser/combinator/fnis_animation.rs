//! - FNIS Animation: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]

use winnow::ascii::{space0, space1};
use winnow::combinator::{repeat, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::anim_types::{
    parse_anim_type, FNISAnimType,
};
use crate::behaviors::tasks::fnis::list_parser::combinator::comment::take_till_line_or_eof;
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::{
    parse_anim_flags, FNISAnimFlagSet, FNISAnimFlags,
};

#[derive(Debug, PartialEq)]
pub struct FNISAnimation<'a> {
    pub anim_type: FNISAnimType,
    pub flag_set: FNISAnimFlagSet<'a>,
    pub anim_event: &'a str,
    pub anim_file: &'a str,
    pub anim_objects: Vec<&'a str>,
}

pub fn parse_fnis_animation<'a>(input: &mut &'a str) -> ModalResult<FNISAnimation<'a>> {
    seq!(FNISAnimation {
            _: space0,
            anim_type: parse_anim_type,
            _: space1,
            flag_set: parse_anim_flags,
            _: space1,
            anim_event: take_till(1.., [' ' , '\t']).context(StrContext::Label("anim_event: str")),
            _: space1,
            anim_file: take_till(1.., [' ' , '\t', '\r', '\n']).context(StrContext::Label("anim_file: str")),
            anim_objects: parse_anim_objects(flag_set.flags).context(StrContext::Label("anim_objects: str")),
            _: take_till_line_or_eof,
    })
    .context(StrContext::Label("FNIS Animation"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]",
    )))
    .parse_next(input)
}

fn parse_anim_objects<'a>(
    flags: FNISAnimFlags,
) -> impl FnMut(&mut &'a str) -> ModalResult<Vec<&'a str>> {
    move |input: &mut &'a str| {
        let has_anim_object = flags.contains(FNISAnimFlags::AnimObjects);
        if !has_anim_object {
            return Ok(vec![]);
        }

        space1.parse_next(input)?;
        repeat(1.., take_till(1.., [' ', '\t', '\r', '\n']))
            .context(StrContext::Label("anim_objects: Vec<str>"))
            .context(StrContext::Expected(StrContextValue::Description(
                "When setting anim_objects, you must use the -o flag. \
                    If -o is used, at least one anim_object is required. \
                    Otherwise, no anim_objects are allowed.",
            )))
            .parse_next(input)
    }
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
                flag_set: FNISAnimFlagSet {
                    flags: FNISAnimFlags::AnimObjects | FNISAnimFlags::Known,
                    ..Default::default()
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
