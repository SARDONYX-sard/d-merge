//! - FNIS Animation: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]

use skyrim_anim_parser::adsf::normal::Translation;
use winnow::ascii::{space0, space1};
use winnow::combinator::{opt, repeat, separated, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::{
    anim_types::{parse_anim_type, FNISAnimType},
    comment::skip_ws_and_comments,
    flags::{parse_anim_flags, FNISAnimFlagSet, FNISAnimFlags},
    motion::parse_md_data,
    rotation::{parse_rd_data, RotationData},
    take_till_fnis_ignores, take_till_space,
};

#[derive(Debug, PartialEq)]
pub struct FNISAnimation<'a> {
    pub anim_type: FNISAnimType,
    pub flag_set: FNISAnimFlagSet<'a>,
    pub anim_event: &'a str,
    pub anim_file: &'a str,
    pub anim_objects: Vec<&'a str>,
    pub motions: Vec<Translation<'a>>,
    pub rotations: Vec<RotationData<'a>>,
}

pub fn parse_fnis_animation<'a>(input: &mut &'a str) -> ModalResult<FNISAnimation<'a>> {
    seq!(FNISAnimation {
            // 1 line
            _: space0,
            anim_type: parse_anim_type,
            flag_set: opt(|input: &mut &'a str| {
                let (flags,) = seq! {
                    _: space1,
                    parse_anim_flags
                }.parse_next(input)?;
                Ok(flags)
            }).map(|set| set.unwrap_or_default()),
            _: space1,
            anim_event: take_till_space.context(StrContext::Label("anim_event: str")),
            _: space1,
            anim_file: take_till_fnis_ignores.context(StrContext::Label("anim_file: str")),
            anim_objects: parse_anim_objects(flag_set.flags).context(StrContext::Label("anim_objects: str")),
            _: skip_ws_and_comments,

            motions: repeat(0.., parse_md_data), // n time lines
            rotations: repeat(0.., parse_rd_data), // n time lines
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
        separated(1.., take_till_fnis_ignores, space1)
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
                anim_objects: vec!["AnimObjectIronSword"],
                motions: vec![],
                rotations: vec![],
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
