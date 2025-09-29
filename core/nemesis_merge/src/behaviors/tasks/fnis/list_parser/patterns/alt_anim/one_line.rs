//! Alternate animations parsing (AAprefix, AAset, T)

use winnow::ascii::{dec_uint, space0, space1, Caseless};
use winnow::combinator::{repeat, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::comment::skip_ws_and_comments;
use crate::behaviors::tasks::fnis::list_parser::combinator::{
    take_till_fnis_ignores, take_till_space, Trigger,
};

/// AlterativeAnimation set
///
/// `AAset <animation_group> <number>`
#[derive(Debug, Clone, PartialEq)]
pub struct AASet<'a> {
    pub group: &'a str,
    pub slots: u64,
}

/// `T <alternate_animation> <trigger1> <time1> ...`
#[derive(Debug, Clone, PartialEq)]
pub struct AnimTrigger<'a> {
    /// animation name
    pub anim_name: &'a str,
    /// Trigger event, time
    pub triggers: Vec<Trigger<'a>>,
}

/// Parse `AAprefix <3_character_mod_abbreviation>`
pub fn parse_alt_anim_prefix_line<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let (prefix,) = seq! {
        _: space0,
        _: Caseless("AAprefix"),
        _: space1,
        take_till_fnis_ignores,
        _: skip_ws_and_comments,
    }
    .context(StrContext::Label("Alternate Animations"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Expected Syntax: `AAprefix <3_character_mod_abbreviation: str>` (e.g. `AAprefix fsm`)",
    )))
    .parse_next(input)?;
    Ok(prefix)
}

/// Parse `AAset <animation_group> <number>`
pub fn parse_alt_anim_set_line<'a>(input: &mut &'a str) -> ModalResult<AASet<'a>> {
    let (anim_group, slots_count) = seq! {
        _: space0,
        _: Caseless("AAset"),
        _: space1,
        take_till(0.., [' ', '\t']),
        _: space1,
        dec_uint,
        _: skip_ws_and_comments,
    }
    .context(StrContext::Label("Alternate Animations"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Expected Syntax: `AAset <animation_group: str> <number: u64>`",
    )))
    .parse_next(input)?;
    Ok(AASet {
        group: anim_group,
        slots: slots_count,
    })
}

/// Parse `T <alternate_animation> <trigger1> <time1> ...`
pub fn parse_alt_anim_trigger_line<'a>(input: &mut &'a str) -> ModalResult<AnimTrigger<'a>> {
    fn parse_trigger<'a>(input: &mut &'a str) -> ModalResult<Trigger<'a>> {
        seq! {
            Trigger {
                event: take_till_space.verify(|s: &str| s.parse::<f32>().is_err()).context(StrContext::Label("Trigger.event: str")),
                _: space1,
                time: take_till_fnis_ignores.verify_map(|s: &str| s.parse::<f32>().ok()).context(StrContext::Label("Trigger.time: f32")),
                _: space0,
            }
        }
        .parse_next(input)
    }

    let (anim_name, triggers) = seq! {
        _: space0,
        _: Caseless("T"),
        _: space1,
        take_till_space.context(StrContext::Label("anim_name: str")),
        _: space0,
        repeat(0.., parse_trigger).context(StrContext::Label("triggers: Vec<Trigger>")),
        _: skip_ws_and_comments,
    }
    .context(StrContext::Label("Alternate Animations"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Expected Syntax: `T <alternate_animation: str> <trigger1: str> <time1: f32> ...`",
    )))
    .parse_next(input)?;
    Ok(AnimTrigger {
        anim_name,
        triggers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::{
        combinator::Trigger,
        test_helpers::{must_fail, must_parse},
    };

    #[test]
    fn test_parse_alt_anim_prefix_line() {
        let res = must_parse(parse_alt_anim_prefix_line, "AAprefix fsm\n");
        assert_eq!(res, "fsm");

        let res2 = must_parse(parse_alt_anim_prefix_line, "   AAprefix xyz\n");
        assert_eq!(res2, "xyz");

        must_fail(parse_alt_anim_prefix_line, "AAprefix\n"); // missing code
    }

    #[test]
    fn test_parse_alt_anim_set_line() {
        let res = must_parse(parse_alt_anim_set_line, "AAset _mt 9\n");
        assert_eq!(
            res,
            AASet {
                group: "_mt",
                slots: 9
            }
        );

        let res2 = must_parse(parse_alt_anim_set_line, "  AAset _run 42\n");
        assert_eq!(
            res2,
            AASet {
                group: "_run",
                slots: 42
            }
        );

        must_fail(parse_alt_anim_set_line, "AAset _mt not_a_number\n");
    }

    #[test]
    fn test_parse_alt_anim_t_line() {
        let res = must_parse(parse_alt_anim_trigger_line, "T _run start 0.0 end 1.5\n");
        assert_eq!(
            res,
            AnimTrigger {
                anim_name: "_run",
                triggers: vec![
                    Trigger {
                        event: "start",
                        time: 0.0
                    },
                    Trigger {
                        event: "end",
                        time: 1.5
                    },
                ],
            }
        );

        let res2 = must_parse(
            parse_alt_anim_trigger_line,
            "  T _walk trigger1 0.2 trigger2 3.5\n",
        );
        assert_eq!(
            res2,
            AnimTrigger {
                anim_name: "_walk",
                triggers: vec![
                    Trigger {
                        event: "trigger1",
                        time: 0.2
                    },
                    Trigger {
                        event: "trigger2",
                        time: 3.5
                    },
                ],
            }
        );

        must_fail(parse_alt_anim_trigger_line, "T _anim trig not_a_float\n");
    }
}
