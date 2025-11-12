//! `FNIS_<mod name>_List.txt` parser
//!
//! See `FNIS for Modders_V6.2.pdf` by fore
pub(crate) mod combinator;
pub(crate) mod patterns;
#[cfg(test)]
mod test_helpers;

use winnow::ascii::Caseless;
use winnow::combinator::{alt, fail, opt};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::anim_var::parse_anim_var_line;
use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;
use crate::behaviors::tasks::fnis::list_parser::combinator::{
    anim_types::{parse_anim_type, FNISAnimType},
    fnis_animation::{parse_fnis_animation, FNISAnimation},
    version::{parse_version_line, Version},
};
use crate::behaviors::tasks::fnis::list_parser::patterns::{
    alt_anim::{parse_alternative_animation, AlternativeAnimation},
    chair::{parse_fnis_chair_animation, FNISChairAnimation},
    furniture::{parse_furniture_animation, FurnitureAnimation},
    pair_and_kill::{parse_paired_animation, FNISPairedAndKillAnimation},
    sequenced::{parse_seq_animation, SequencedAnimation},
};

#[derive(Debug, PartialEq)]
pub(crate) enum SyntaxPattern<'a> {
    AltAnim(AlternativeAnimation<'a>),
    AnimObject(FNISAnimation<'a>),
    Basic(FNISAnimation<'a>),
    Chair(FNISChairAnimation<'a>),
    Furniture(FurnitureAnimation<'a>),
    OffsetArm(FNISAnimation<'a>),
    PairAndKillMove(FNISPairedAndKillAnimation<'a>),
    Sequenced(SequencedAnimation<'a>),

    AnimVar(combinator::anim_var::AnimVar<'a>),
}

/// One mod FNIS_<mod namespace>_List.txt
#[derive(Debug, PartialEq)]
pub(crate) struct FNISList<'a> {
    /// Mod version
    pub version: Option<Version>,

    /// sequenced animations
    pub(crate) patterns: Vec<SyntaxPattern<'a>>,
}

pub fn parse_fnis_list<'a>(input: &mut &'a str) -> ModalResult<FNISList<'a>> {
    skip_ws_and_comments.parse_next(input)?;

    let version = opt(parse_version_line).parse_next(input)?;

    skip_ws_and_comments.parse_next(input)?;

    let mut patterns = vec![];

    while let Ok((_, anim_type)) = alt((
        parse_anim_type,
        Caseless("AAprefix").value(FNISAnimType::Alternate),
        Caseless("AnimVar").value(FNISAnimType::AnimVar),
    ))
    .parse_peek(input)
    {
        // FIXME: Need validate OffsetArm
        let pattern = match anim_type {
            FNISAnimType::AnimVar => parse_anim_var_line
                .map(SyntaxPattern::AnimVar)
                .parse_next(input)?,
            FNISAnimType::Basic => parse_fnis_animation
                .map(SyntaxPattern::Basic)
                .parse_next(input)?,
            FNISAnimType::AnimObject => parse_fnis_animation
                .map(SyntaxPattern::AnimObject)
                .parse_next(input)?,
            FNISAnimType::OffsetArm => parse_fnis_animation
                .map(SyntaxPattern::OffsetArm)
                .parse_next(input)?,
            FNISAnimType::Sequenced | FNISAnimType::SequencedOptimized => parse_seq_animation
                .map(SyntaxPattern::Sequenced)
                .parse_next(input)?,

            FNISAnimType::Furniture | FNISAnimType::FurnitureOptimized => parse_furniture_animation
                .map(SyntaxPattern::Furniture)
                .parse_next(input)?,

            FNISAnimType::SequencedContinued => fail
                .context(StrContext::Expected(StrContextValue::Description(
                    r"SequencedContinued ('+') must follow s/so/fu/fuo.
Example of correct usage:
    s AnimEvent Anim.hkx
    + ContinuedEvent Continued.hkx
",
                )))
                .parse_next(input)?,

            FNISAnimType::Paired | FNISAnimType::KillMove => parse_paired_animation
                .map(SyntaxPattern::PairAndKillMove)
                .parse_next(input)?,

            FNISAnimType::Alternate => parse_alternative_animation
                .map(SyntaxPattern::AltAnim)
                .parse_next(input)?,

            FNISAnimType::Chair => parse_fnis_chair_animation
                .map(SyntaxPattern::Chair)
                .parse_next(input)?,
        };

        patterns.push(pattern);
    }

    Ok(FNISList { version, patterns })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::must_parse;

    #[test]
    #[ignore]
    fn test_list() {
        let list = std::fs::read_to_string("../../dummy/fnis_test_mods/FNIS Zoo 5.0.1/Meshes/actors/dlc02/riekling/animations/FNISZoo/FNIS_FNISZoo_riekling_List.txt").unwrap();
        let ret = must_parse(parse_fnis_list, &list);
        std::fs::write("./debug.log", format!("{ret:#?}")).unwrap();
    }
}
