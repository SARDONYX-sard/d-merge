//! - FNIS Alterative Animation
pub mod one_line;

use winnow::combinator::repeat;
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::patterns::alt_anim::one_line::{
    parse_alt_anim_prefix_line, parse_alt_anim_set_line, parse_alt_anim_trigger_line, AASet,
    AnimTrigger,
};

/// Alterative Animation
#[derive(Debug, PartialEq)]
pub struct AlternativeAnimation<'a> {
    pub prefix: &'a str,
    pub set: Vec<AASet<'a>>,
    pub trigger: Vec<AnimTrigger<'a>>,
}

pub fn parse_alternative_animation<'a>(
    input: &mut &'a str,
) -> ModalResult<AlternativeAnimation<'a>> {
    parse_furniture_animations_inner
        .context(StrContext::Label("Alterative Animation"))
        .context(StrContext::Expected(StrContextValue::Description("")))
        .parse_next(input)
}

fn parse_furniture_animations_inner<'a>(
    input: &mut &'a str,
) -> ModalResult<AlternativeAnimation<'a>> {
    let prefix = parse_alt_anim_prefix_line.parse_next(input)?;
    let set = repeat(1.., parse_alt_anim_set_line).parse_next(input)?;
    let trigger = repeat(1.., parse_alt_anim_trigger_line).parse_next(input)?;

    Ok(AlternativeAnimation {
        prefix,
        set,
        trigger,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::{
        combinator::Trigger, test_helpers::must_parse,
    };

    #[test]
    fn test_parse_fnis_animation_valid() {
        let parsed = must_parse(
            parse_alternative_animation,
            r"AAprefix fsm
AAset _mt 9
AAset _boweqp 1
AAset _bowidle 1

T test.hkx Test_Event -0.100 Test_trigger -0.00 Test_trigger2 3.00
T test2.hkx Test2_Event -0.100 Test2_trigger -0.00 Test2_trigger2 3.00
",
        );

        let expected = AlternativeAnimation {
            prefix: "fsm",
            set: vec![
                AASet {
                    group: "_mt",
                    slots: 9,
                },
                AASet {
                    group: "_boweqp",
                    slots: 1,
                },
                AASet {
                    group: "_bowidle",
                    slots: 1,
                },
            ],
            trigger: vec![
                AnimTrigger {
                    anim_name: "test.hkx",
                    triggers: vec![
                        Trigger {
                            event: "Test_Event",
                            time: -0.1,
                        },
                        Trigger {
                            event: "Test_trigger",
                            time: -0.0,
                        },
                        Trigger {
                            event: "Test_trigger2",
                            time: 3.0,
                        },
                    ],
                },
                AnimTrigger {
                    anim_name: "test2.hkx",
                    triggers: vec![
                        Trigger {
                            event: "Test2_Event",
                            time: -0.1,
                        },
                        Trigger {
                            event: "Test2_trigger",
                            time: -0.0,
                        },
                        Trigger {
                            event: "Test2_trigger2",
                            time: 3.0,
                        },
                    ],
                },
            ],
        };

        assert_eq!(parsed, expected);
    }
}
