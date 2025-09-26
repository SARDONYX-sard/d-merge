//! Alternate animations parsing (AAprefix, AAset, T)

use winnow::ascii::{line_ending, space0, space1, till_line_ending, Caseless};
use winnow::combinator::{repeat_till, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

/// Represents a single line of an alternate animation definition.
#[derive(Debug, Clone, PartialEq)]
pub enum AltAnimLine<'a> {
    /// `AAprefix <3_character_mod_abbreviation>`
    Prefix(&'a str),
    /// `AAset <animation_group> <number>`
    Set { group: &'a str, slots: u64 },
    /// `T <alternate_animation> <trigger1> <time1> ...`
    Trigger {
        anim: &'a str,
        /// Trigger event, time
        triggers: Vec<Trigger<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger<'a> {
    pub event: &'a str,
    pub time: &'a str,
}

/// Parse `AAprefix <3_character_mod_abbreviation>`
pub fn parse_alt_anim_prefix_line<'a>(input: &mut &'a str) -> ModalResult<AltAnimLine<'a>> {
    let (prefix,) = seq! {
        _: space0,
        _: Caseless("AAprefix"),
        _: space0,
        till_line_ending.verify(|s: &str| !s.is_empty()),
        _: line_ending,
    }
    .context(StrContext::Label("Alternate Animations"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Expected Syntax: `AAprefix <3_character_mod_abbreviation: str>` (e.g. `AAprefix fsm`)",
    )))
    .parse_next(input)?;
    Ok(AltAnimLine::Prefix(prefix.trim()))
}

/// Parse `AAset <animation_group> <number>`
pub fn parse_alt_anim_set_line<'a>(input: &mut &'a str) -> ModalResult<AltAnimLine<'a>> {
    let (anim_group, slots_count) = seq! {
        _: space0,
        _: Caseless("AAset"),
        _: space1,
        take_till(0.., [' ', '\t']),
        _: space1,
        till_line_ending.verify_map(|s: &str| s.parse::<u64>().ok()),
        _: line_ending,
    }
    .context(StrContext::Label("Alternate Animations"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Expected Syntax: `AAset <animation_group: str> <number: u64>`",
    )))
    .parse_next(input)?;
    Ok(AltAnimLine::Set {
        group: anim_group,
        slots: slots_count,
    })
}

/// Parse `T <alternate_animation> <trigger1> <time1> ...`
pub fn parse_alt_anim_t_line<'a>(input: &mut &'a str) -> ModalResult<AltAnimLine<'a>> {
    fn parse_trigger<'a>(input: &mut &'a str) -> ModalResult<Trigger<'a>> {
        seq! {
            Trigger {
                event: take_till(0.., [' ', '\t']).verify(|s: &str| s.parse::<f32>().is_err()).context(StrContext::Label("Trigger.event: str")),
                _: space1,
                time: take_till(0.., [' ', '\t', '\r', '\n']).verify(|s: &str| s.parse::<f32>().is_ok()).context(StrContext::Label("Trigger.time: f32")),
                _: space0,
            }
        }
        .parse_next(input)
    }

    let (anim_anim, (triggers, _)) = seq! {
        _: space0,
        _: Caseless("T"),
        _: space1,
        take_till(0.., [' ', '\t']),
        _: space1,
        repeat_till(0.., parse_trigger, line_ending),
    }
    .context(StrContext::Label("Alternate Animations"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Expected Syntax: `T <alternate_animation: str> <trigger1: str> <time1: f32> ...`",
    )))
    .parse_next(input)?;
    Ok(AltAnimLine::Trigger {
        anim: anim_anim,
        triggers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn test_parse_alt_anim_prefix_line() {
        let res = must_parse(parse_alt_anim_prefix_line, "AAprefix fsm\n");
        assert_eq!(res, AltAnimLine::Prefix("fsm"));

        let res2 = must_parse(parse_alt_anim_prefix_line, "   AAprefix xyz\n");
        assert_eq!(res2, AltAnimLine::Prefix("xyz"));

        must_fail(parse_alt_anim_prefix_line, "AAprefix\n"); // missing code
    }

    #[test]
    fn test_parse_alt_anim_set_line() {
        let res = must_parse(parse_alt_anim_set_line, "AAset _mt 9\n");
        assert_eq!(
            res,
            AltAnimLine::Set {
                group: "_mt",
                slots: 9
            }
        );

        let res2 = must_parse(parse_alt_anim_set_line, "  AAset _run 42\n");
        assert_eq!(
            res2,
            AltAnimLine::Set {
                group: "_run",
                slots: 42
            }
        );

        must_fail(parse_alt_anim_set_line, "AAset _mt not_a_number\n");
    }

    #[test]
    fn test_parse_alt_anim_t_line() {
        let res = must_parse(parse_alt_anim_t_line, "T _run start 0.0 end 1.5\n");
        assert_eq!(
            res,
            AltAnimLine::Trigger {
                anim: "_run",
                triggers: vec![
                    Trigger {
                        event: "start",
                        time: "0.0"
                    },
                    Trigger {
                        event: "end",
                        time: "1.5"
                    },
                ],
            }
        );

        let res2 = must_parse(
            parse_alt_anim_t_line,
            "  T _walk trigger1 0.2 trigger2 3.5\n",
        );
        assert_eq!(
            res2,
            AltAnimLine::Trigger {
                anim: "_walk",
                triggers: vec![
                    Trigger {
                        event: "trigger1",
                        time: "0.2"
                    },
                    Trigger {
                        event: "trigger2",
                        time: "3.5"
                    },
                ],
            }
        );

        must_fail(parse_alt_anim_t_line, "T _anim trig not_a_float\n");
    }
}
