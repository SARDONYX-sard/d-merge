//! - FNIS Sequenced Animation: s|so [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]

use winnow::combinator::fail;
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::fnis_animation::{
    parse_fnis_animation, FNISAnimation,
};

/// Furniture animation
#[derive(Debug, PartialEq)]
pub struct FurnitureAnimation<'a> {
    /// furniture animations
    pub animations: Vec<FNISAnimation<'a>>,
}

pub fn parse_furniture_animation<'a>(input: &mut &'a str) -> ModalResult<FurnitureAnimation<'a>> {
    parse_furniture_animations_inner
        .context(StrContext::Label("Furniture Animation"))
        .context(StrContext::Expected(StrContextValue::Description(
            r"- The first animation must be Furniture/FurnitureOptimized and acyclic (-a).
- The last animation must be acyclic (-a).
- The second-to-last animation should be cyclic (not -a).

# Example
fu -a Kneel_Enter Kneel_Enter.hkx
+ -o,B1.2 Kneel_Loop1 Kneel_Loop1.hkx myAnimObject1 myAnimObject2
+ -o Kneel_Loop2 Kneel_Loop2.hkx myAnimObject1 myAnimObject2
+ -a,o Kneel_Exit Kneel_Exit.hkx myAnimObject1 myAnimObject2",
        )))
        .parse_next(input)
}

fn parse_furniture_animations_inner<'a>(
    input: &mut &'a str,
) -> ModalResult<FurnitureAnimation<'a>> {
    use crate::behaviors::tasks::fnis::list_parser::combinator::{
        anim_types::FNISAnimType::*, flags::FNISAnimFlags,
    };

    let seq_start_anim = parse_fnis_animation
        .verify(|anim| {
            let is_furniture = matches!(anim.anim_type, Furniture | FurnitureOptimized);
            let is_acyclic = anim.flag_set.flags.contains(FNISAnimFlags::Acyclic);
            is_furniture && is_acyclic
        })
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

    if animations.len() < 3 {
        return fail.parse_next(input)?;
    }

    if let Some(last) = animations.last() {
        if !last.flag_set.flags.contains(FNISAnimFlags::Acyclic) {
            return fail.parse_next(input)?;
        }
    };

    // Omitted since already checked initially
    {
        let lasts = &animations[animations.len() - 2..animations.len()]; // Second to last
        let is_last_second_acyclic = lasts[0].flag_set.flags.contains(FNISAnimFlags::Acyclic);
        let is_last_acyclic = lasts[1].flag_set.flags.contains(FNISAnimFlags::Acyclic);
        if is_last_second_acyclic && !is_last_acyclic {
            return fail.parse_next(input)?;
        }
    }

    Ok(FurnitureAnimation { animations })
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
            parse_furniture_animation,
            r"fu -a Kneel_Enter Kneel_Enter.hkx
+ -o,B1.2 Kneel_Loop1 Kneel_Loop1.hkx myAnimObject1 myAnimObject2
+ -o Kneel_Loop2 Kneel_Loop2.hkx myAnimObject1 myAnimObject2
+ -a,o Kneel_Exit Kneel_Exit.hkx myAnimObject1 myAnimObject2",
        );

        let expected = FurnitureAnimation {
            animations: vec![
                FNISAnimation {
                    anim_type: FNISAnimType::Furniture,
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::Acyclic,
                        blend_time: None,
                        triggers: vec![],
                        anim_vars: vec![],
                    },
                    anim_event: "Kneel_Enter",
                    anim_file: "Kneel_Enter.hkx",
                    anim_objects: vec![],
                    motions: vec![],
                    rotations: vec![],
                },
                FNISAnimation {
                    anim_type: FNISAnimType::SequencedContinued,
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::AnimObjects,
                        blend_time: Some(1.2),
                        triggers: vec![],
                        anim_vars: vec![],
                    },
                    anim_event: "Kneel_Loop1",
                    anim_file: "Kneel_Loop1.hkx",
                    anim_objects: vec!["myAnimObject1", "myAnimObject2"],
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
                    anim_event: "Kneel_Loop2",
                    anim_file: "Kneel_Loop2.hkx",
                    anim_objects: vec!["myAnimObject1", "myAnimObject2"],
                    motions: vec![],
                    rotations: vec![],
                },
                FNISAnimation {
                    anim_type: FNISAnimType::SequencedContinued,
                    flag_set: FNISAnimFlagSet {
                        flags: FNISAnimFlags::Acyclic | FNISAnimFlags::AnimObjects,
                        blend_time: None,
                        triggers: vec![],
                        anim_vars: vec![],
                    },
                    anim_event: "Kneel_Exit",
                    anim_file: "Kneel_Exit.hkx",
                    anim_objects: vec!["myAnimObject1", "myAnimObject2"],
                    motions: vec![],
                    rotations: vec![],
                },
            ],
        };

        assert_eq!(parsed, expected);
    }
}
