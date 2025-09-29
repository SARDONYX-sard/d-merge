//! - FNIS Sequenced Animation: s|so [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]

use winnow::combinator::fail;
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::fnis_animation::{
    parse_fnis_animation, FNISAnimation,
};

/// sequenced animations
#[derive(Debug, PartialEq)]
pub struct SequencedAnimation<'a> {
    /// sequenced animations
    pub animations: Vec<FNISAnimation<'a>>,
}

pub fn parse_seq_animation<'a>(input: &mut &'a str) -> ModalResult<SequencedAnimation<'a>> {
    use crate::behaviors::tasks::fnis::list_parser::combinator::anim_types::FNISAnimType::{
        Sequenced, SequencedContinued, SequencedOptimized,
    };

    let seq_start_anim = parse_fnis_animation
        .verify(|anim| matches!(anim.anim_type, Sequenced | SequencedOptimized))
        .parse_next(input)?;

    let mut animations = vec![seq_start_anim];

    // NOTE: To avoid intermediate allocations of `Vec` caused by using `repeat`, manually perform the loop.
    loop {
        match parse_fnis_animation
            .verify(|anim| matches!(anim.anim_type, SequencedContinued))
            .parse_next(input)
        {
            Ok(anim) => animations.push(anim),
            Err(winnow::error::ErrMode::Backtrack(_)) => break, // End if no further action.
            Err(e) => return Err(e),
        }
    }

    if animations.len() < 2 {
        return fail
            .context(StrContext::Expected(StrContextValue::Description(
                "Sequenced Animation requires at least 2 consecutive animations.",
            )))
            .parse_next(input)?;
    }
    // TODO: Once I understand it, I should perform verification here.
    // It seems seq anims must always have acyclic(`-a`) appended at the end.
    // However, the sample has `-a` at the beginning.
    // I don't quite understand this.

    Ok(SequencedAnimation { animations })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::{
        combinator::{
            anim_types::FNISAnimType,
            flags::{FNISAnimFlagSet, FNISAnimFlags},
        },
        test_helpers::must_parse,
    };

    #[test]
    fn test_parse_fnis_animation_valid() {
        let parsed = must_parse(
            parse_seq_animation,
            r"s -a,k MyCheerSA1 ..\idlewave.hkx
+ -o MyCheerSA2 MyCheerAnim1.hkx AnimObjectIronSword
+ -o,k MyCheerSA3 MyCheerAnim2.hkx AnimObjectIronSword",
        );

        let expected = SequencedAnimation {
            animations: vec![
                FNISAnimation {
                    anim_type: FNISAnimType::Sequenced,
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::Acyclic | FNISAnimFlags::Known,
                        blend_time: None,
                        triggers: vec![],
                        anim_vars: vec![],
                    },
                    anim_event: "MyCheerSA1",
                    anim_file: "..\\idlewave.hkx",
                    anim_objects: vec![],
                    motions: vec![],
                    rotations: vec![],
                },
                FNISAnimation {
                    anim_type: FNISAnimType::SequencedContinued,
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::AnimObjects,
                        blend_time: None,
                        triggers: vec![],
                        anim_vars: vec![],
                    },
                    anim_event: "MyCheerSA2",
                    anim_file: "MyCheerAnim1.hkx",
                    anim_objects: vec!["AnimObjectIronSword"],
                    motions: vec![],
                    rotations: vec![],
                },
                FNISAnimation {
                    anim_type: FNISAnimType::SequencedContinued,
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::AnimObjects | FNISAnimFlags::Known,
                        blend_time: None,
                        triggers: vec![],
                        anim_vars: vec![],
                    },
                    anim_event: "MyCheerSA3",
                    anim_file: "MyCheerAnim2.hkx",
                    anim_objects: vec!["AnimObjectIronSword"],
                    motions: vec![],
                    rotations: vec![],
                },
            ],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    #[ignore]
    fn test_list() {
        use crate::behaviors::tasks::fnis::list_parser::combinator::version::parse_version_line;

        let list = std::fs::read_to_string("../../dummy/fnis_test_mods/FNIS Flyer SE 7.0/Data/Meshes/actors/character/animations/FNISFlyer/FNIS_FNISFLyer_List.txt").unwrap();
        let ret = must_parse(
            (
                parse_version_line,
                parse_seq_animation,
                parse_fnis_animation,
            ),
            &list,
        );
        std::fs::write("./debug.log", format!("{ret:#?}")).unwrap();
    }
}
