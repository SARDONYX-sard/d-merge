//! Pair and KillMove animation flags parsing.
//!
//! Supported flags:
//! - Simple flags: `bsa`, `h`, `k`, `o`
//! - Duration: `D<time>` (mandatory, e.g. `D1.5`)
//! - Triggers:
//!   - `T<event>/<time>` → stored in `triggers`
//!   - `T2_<event>/<time>` → stored in `triggers2` (event string keeps the `2_` prefix)

use winnow::ascii::{float, space0, Caseless};
use winnow::combinator::{alt, fail, opt, preceded};
use winnow::error::{StrContext, StrContextValue};
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::flags::{
    parse_trigger_options, FNISAnimFlags,
};
use crate::behaviors::tasks::fnis::list_parser::combinator::Trigger;

/// Combination of simple bitflags and parameterized flags.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FNISPairAndKillMoveAnimFlagSet<'a> {
    /// Collection of simple on/off flags.
    pub flags: FNISAnimFlags,

    /// Animation duration (e.g. `D1.5`).
    pub duration: f32,

    /// Triggers of the form `T<event>/<time>`.
    pub triggers: Vec<Trigger<'a>>,

    /// Triggers of the form `T2_<event>/<time>`.
    /// Note: the stored `event` string includes the `"2_"` prefix.
    pub triggers2: Vec<Trigger<'a>>,
}

/// Internal representation for parser results.
#[derive(Debug)]
enum ParsedFlag<'a> {
    Simple(FNISAnimFlags),
    /// Blend time in seconds (e.g. `D1.5`).
    DurationTime(f32),
    /// Trigger event of the form `T<event>/<time>`.
    Trigger(Trigger<'a>),
    /// Trigger event of the form `T2_<event>/<time>`.
    Trigger2(Trigger<'a>),
}

/// Parse a list of animation flags separated by commas.
///
/// # Errors
/// Pair and KillMove animations require a duration specified by `D<time>`.
/// If this is missing, the parser will return an error.
pub fn parse_anim_flags<'a>(
    input: &mut &'a str,
) -> ModalResult<FNISPairAndKillMoveAnimFlagSet<'a>> {
    preceded("-", __parse_anim_flags)
        .context(StrContext::Label("FNISPairAndKillMoveAnimFlags"))
        .context(StrContext::Expected(StrContextValue::Description(
            "One of: bsa, h, o, D<time: f32> (e.g. `D1.5`), \
             T<trigger>/<time> (e.g. `TJump/2.0`), \
             T2_<trigger>/<time> (e.g. `T2_killactor/3.333`), \
             <AnimObject>/<1 or 2>",
        )))
        .parse_next(input)
}

/// Parse a list of animation flags separated by commas.
fn __parse_anim_flags<'a>(input: &mut &'a str) -> ModalResult<FNISPairAndKillMoveAnimFlagSet<'a>> {
    let mut set = FNISPairAndKillMoveAnimFlagSet::default();
    let mut has_duration = false;

    loop {
        match parse_anim_flag.parse_next(input)? {
            ParsedFlag::Simple(flag) => set.flags |= flag,
            ParsedFlag::DurationTime(duration) => {
                set.duration = duration;
                has_duration = true;
            }
            ParsedFlag::Trigger(trigger) => set.triggers.push(trigger),
            ParsedFlag::Trigger2(trigger) => set.triggers2.push(trigger),
        }

        // intended `,` separator
        if opt((space0, ',')).parse_next(input)?.is_some() {
            space0.parse_next(input)?;
            continue;
        }
        break;
    }

    if !has_duration {
        fail.context(StrContext::Expected(StrContextValue::Description(
            "missing duration flag: pair and killMoves animations require one (format: D<time>, e.g. D1.5)",
        )))
        .parse_next(input)?;
    }

    Ok(set)
}

/// Parse a single animation flag (simple or parameterized).
fn parse_anim_flag<'a>(input: &mut &'a str) -> ModalResult<ParsedFlag<'a>> {
    alt((
        parse_anim_flag_simple.map(ParsedFlag::Simple),
        parse_anim_flag_param,
    ))
    .parse_next(input)
}

fn parse_anim_flag_simple(input: &mut &str) -> ModalResult<FNISAnimFlags> {
    alt((
        "bsa".value(FNISAnimFlags::BSA),
        "h".value(FNISAnimFlags::HeadTracking),
        "k".value(FNISAnimFlags::Known),
        "o".value(FNISAnimFlags::AnimObjects),
    ))
    .parse_next(input)
}

fn parse_anim_flag_param<'a>(input: &mut &'a str) -> ModalResult<ParsedFlag<'a>> {
    alt((
        preceded(Caseless("D"), float).map(ParsedFlag::DurationTime),
        parse_trigger_flag,
    ))
    .parse_next(input)
}

/// Parse trigger flags (`T...` or `T2_...`).
///
/// - `TJump/2.0` → `ParsedFlag::Trigger`
/// - `T2_killactor/3.333` → `ParsedFlag::Trigger2` (event includes `"2_"`)
fn parse_trigger_flag<'a>(input: &mut &'a str) -> ModalResult<ParsedFlag<'a>> {
    parse_trigger_options
        .map(|trig: Trigger<'a>| {
            if trig.event.starts_with("2_") {
                ParsedFlag::Trigger2(trig)
            } else {
                ParsedFlag::Trigger(trig)
            }
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn parse_simple_flags() {
        assert_eq!(
            must_parse(parse_anim_flag_simple, "bsa"),
            FNISAnimFlags::BSA
        );
        assert_eq!(
            must_parse(parse_anim_flag_simple, "h"),
            FNISAnimFlags::HeadTracking
        );
        assert_eq!(
            must_parse(parse_anim_flag_simple, "o"),
            FNISAnimFlags::AnimObjects
        );
    }

    #[test]
    fn parse_duration_flag() {
        match must_parse(parse_anim_flag_param, "D1.5") {
            ParsedFlag::DurationTime(v) => assert!((v - 1.5).abs() < f32::EPSILON),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_trigger_flag_t() {
        let input = "-D2.0,TJump/2.0";
        let parsed = must_parse(parse_anim_flags, input);

        assert_eq!(
            parsed,
            FNISPairAndKillMoveAnimFlagSet {
                flags: FNISAnimFlags::default(),
                duration: 2.0,
                triggers: vec![Trigger {
                    event: "Jump",
                    time: 2.0,
                }],
                triggers2: vec![],
            }
        );
    }

    #[test]
    fn parse_trigger_flag_t2() {
        let input = "-D2.0,T2_killactor/3.333";
        let parsed = must_parse(parse_anim_flags, input);

        assert_eq!(
            parsed,
            FNISPairAndKillMoveAnimFlagSet {
                flags: FNISAnimFlags::empty(),
                duration: 2.0,
                triggers: vec![],
                triggers2: vec![Trigger {
                    event: "2_killactor",
                    time: 3.333,
                }],
            }
        );
    }

    #[test]
    fn parse_full_flagset() {
        let input = "-bsa,D1.0,TJump/1.0,T2_killactor/3.333";
        let parsed = must_parse(parse_anim_flags, input);

        assert_eq!(
            parsed,
            FNISPairAndKillMoveAnimFlagSet {
                flags: FNISAnimFlags::BSA,
                duration: 1.0,
                triggers: vec![Trigger {
                    event: "Jump",
                    time: 1.0,
                }],
                triggers2: vec![Trigger {
                    event: "2_killactor",
                    time: 3.333,
                }],
            }
        );
    }

    #[test]
    fn fail_without_duration() {
        must_fail(parse_anim_flags, "-bsa,TJump/1.0");
    }

    #[test]
    fn fail_invalid_flag() {
        must_fail(parse_anim_flag_simple, "invalid");
        must_fail(parse_anim_flag_param, "Xfoo");
    }
}
