//! Animation flags parsing: simple flags and parameterized flags

use winnow::ascii::{alphanumeric1, float, space0, Caseless};
use winnow::combinator::{alt, opt, preceded, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser};

use crate::behaviors::tasks::fnis::list_parser::combinator::Trigger;

/// Combination of simple bitflags and parameterized flags.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FNISAnimFlagSet<'a> {
    /// Collection of simple on/off flags.
    pub flags: FNISAnimFlags,
    /// Blend time in seconds (e.g. `B1.5`).
    pub blend_time: Option<f32>,

    /// Trigger event at given time (e.g. `TJump/2.0`).
    pub triggers: Vec<Trigger<'a>>,
    /// Animation variable set/inverse (e.g. `AVfoo`, `AVIbar`).
    pub anim_vars: Vec<AnimVar<'a>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnimVar<'a> {
    name: &'a str,
    inverse: bool,
}

bitflags::bitflags! {
    /// FNIS animation modifier flags from `<option>` syntax.
    ///
    /// **based on and quoted from** _Fore's_ **"FNIS for Modders_V6.2.pdf"(© Fore)**,
    /// which is part of the FNIS (Fores New Idles in Skyrim) modding documentation.
    #[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
    pub struct FNISAnimFlags: u32 {
        /// No special options.
        const NONE = 0;
        /// **a** – Acyclic: plays only once (default is cyclic loop).
        const Acyclic = 1;
        /// **o** – Animation uses one or more AnimObjects.
        const AnimObjects = 1 << 1;
        /// **ac** – Animated Camera: allows camera control via `Camera3rd [Cam3]` bone.
        const AnimatedCamera = 1 << 2;
        /// **ac1** – Animated Camera Set: enable animated camera at animation start.
        const AnimatedCameraSet = 1 << 3;
        /// **ac0** – Animated Camera Reset: disable animated camera at animation end.
        const AnimatedCameraReset = 1 << 4;
        /// **bsa** – Animation file is part of a BSA archive (excluded from consistency check).
        const BSA = 1 << 5;
        /// **h** – Headtracking ON (default is OFF).
        const HeadTracking = 1 << 6;
        /// **k** – "Known" animation (vanilla or from another mod; excluded from consistency check).
        const Known = 1 << 7;
        /// **md** — Motion is driven by actor AI instead of animation data.
        ///
        /// - "motion driven" = motion from actor's package or player input.
        /// - "animation driven" = motion from animation's motion data.
        /// Most animations default to "animation driven" which disables AI movement.
        /// Use `-md` to keep AI movement active.
        const MotionDriven = 1 << 8;
        /// **st** —  sticky AO. Animation Object will not be unequipped at the end of animation.
        const Sticky = 1 << 9;
        /// **Tn** — Character keeps position after `-a` animation (no IdleForceDefaultState).
        const TransitionNext = 1 << 10;
    }
}

// Internal representation for parser results:
// either a simple bitflags or a parameterized flag.
#[derive(Debug, PartialEq)]
enum ParsedFlag<'a> {
    Simple(FNISAnimFlags),

    BlendTime(f32),
    Trigger(Trigger<'a>),
    AnimVar(AnimVar<'a>),
}

/// Parse a list of animation flags separated by commas.
pub fn parse_anim_flags<'a>(input: &mut &'a str) -> ModalResult<FNISAnimFlagSet<'a>> {
    preceded("-", __parse_anim_flags)
    .context(StrContext::Label("FNISAnimFlags"))
    .context(StrContext::Expected(StrContextValue::Description(
        "One of: ac0, ac1, ac, bsa, md, st, Tn, a, h, k, o, B<n>.<m> (e.g. `B1.5`), T<trigger>/<time> (e.g. `TJump/2.0`), AV<Var name>(e.g. `AVfoo`), AVI<Var name>",
    )))
    .parse_next(input)
}

/// Parse a list of animation flags separated by commas.
fn __parse_anim_flags<'a>(input: &mut &'a str) -> ModalResult<FNISAnimFlagSet<'a>> {
    let mut set = FNISAnimFlagSet::default();
    loop {
        match parse_anim_flag.parse_next(input)? {
            ParsedFlag::Simple(flag) => set.flags |= flag,
            ParsedFlag::BlendTime(time) => set.blend_time = Some(time), // FIXME?: overwrite is_err?
            ParsedFlag::Trigger(trigger) => set.triggers.push(trigger),
            ParsedFlag::AnimVar(anim_var) => set.anim_vars.push(anim_var),
        }

        // Intended `md ,`
        if opt((space0, ',')).parse_next(input)?.is_some() {
            space0.parse_next(input)?;
            continue;
        }
        break;
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
        "ac0".value(FNISAnimFlags::AnimatedCameraReset),
        "ac1".value(FNISAnimFlags::AnimatedCameraSet),
        "bsa".value(FNISAnimFlags::BSA),
        "ac".value(FNISAnimFlags::AnimatedCamera),
        "md".value(FNISAnimFlags::MotionDriven),
        "st".value(FNISAnimFlags::Sticky),
        "Tn".value(FNISAnimFlags::TransitionNext),
        "a".value(FNISAnimFlags::Acyclic),
        "h".value(FNISAnimFlags::HeadTracking),
        "k".value(FNISAnimFlags::Known),
        "o".value(FNISAnimFlags::AnimObjects),
    ))
    .parse_next(input)
}

fn parse_anim_flag_param<'a>(input: &mut &'a str) -> ModalResult<ParsedFlag<'a>> {
    alt((
        seq! {AnimVar{
            _: Caseless("AV"),
            inverse: opt(Caseless("I")).map(|inverse| inverse.is_some()),
            name: alphanumeric1
        }}
        .map(ParsedFlag::AnimVar),
        seq! {ParsedFlag::BlendTime(
            _: Caseless("B"),
            float
        )},
        parse_trigger_options.map(ParsedFlag::Trigger),
    ))
    .parse_next(input)
}

pub fn parse_trigger_options<'a>(input: &mut &'a str) -> ModalResult<Trigger<'a>> {
    seq! {Trigger {
        _: Caseless("T"),
        event: take_till(1.., '/'),
        _: "/",
        time: float,
    }}
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviors::tasks::fnis::list_parser::test_helpers::{must_fail, must_parse};

    #[test]
    fn parse_single_simple_flag() {
        assert_eq!(
            must_parse(parse_anim_flag_simple, "a"),
            FNISAnimFlags::Acyclic
        );
        assert_eq!(
            must_parse(parse_anim_flag_simple, "ac0"),
            FNISAnimFlags::AnimatedCameraReset
        );
        assert_eq!(
            must_parse(parse_anim_flag_simple, "bsa"),
            FNISAnimFlags::BSA
        );
        assert_eq!(
            must_parse(parse_anim_flag_simple, "Tn"),
            FNISAnimFlags::TransitionNext
        );
    }

    #[test]
    fn parse_simple_flag_fail() {
        must_fail(parse_anim_flag_simple, "invalid");
        must_fail(parse_anim_flag_simple, "b1");
    }

    #[test]
    fn parse_parameterized_flags() {
        assert_eq!(
            must_parse(parse_anim_flag_param, "B2.5"),
            ParsedFlag::BlendTime(2.5)
        );

        assert_eq!(
            must_parse(parse_anim_flag_param, "TJump/1.0"),
            ParsedFlag::Trigger(Trigger {
                event: "Jump",
                time: 1.0
            })
        );

        assert_eq!(
            must_parse(parse_anim_flag_param, "AVfoo"),
            ParsedFlag::AnimVar(AnimVar {
                name: "foo",
                inverse: false
            })
        );
        assert_eq!(
            must_parse(parse_anim_flag_param, "AVbar"),
            ParsedFlag::AnimVar(AnimVar {
                name: "bar",
                inverse: true,
            })
        );
    }

    #[test]
    fn parse_parameterized_flag_fail() {
        must_fail(parse_anim_flag_param, "Babc");
        must_fail(parse_anim_flag_param, "TJump/xyz");
        must_fail(parse_anim_flag_param, "AV"); // missing name
    }

    // -----------------------------
    // Combined flags parsing
    // -----------------------------
    #[test]
    fn parse_multiple_flags() {
        let actual = must_parse(parse_anim_flags, "-a,ac0,B1.5,TJump/2.0,AVfoo,AVIbar");
        let expected = FNISAnimFlagSet {
            flags: FNISAnimFlags::Acyclic | FNISAnimFlags::AnimatedCameraReset,
            blend_time: Some(1.5),
            triggers: vec![Trigger {
                event: "Jump",
                time: 2.0,
            }],
            anim_vars: vec![
                AnimVar {
                    name: "foo",
                    inverse: false,
                },
                AnimVar {
                    name: "bar",
                    inverse: true,
                },
            ],
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_multiple_flags_with_spaces() {
        let actual = must_parse(parse_anim_flags, "-Tn , md , B1.0 , TRun/3.0 , AVx , AVIy");

        let expected = FNISAnimFlagSet {
            flags: FNISAnimFlags::TransitionNext | FNISAnimFlags::MotionDriven,
            blend_time: Some(1.0),
            triggers: vec![Trigger {
                event: "Run",
                time: 3.0,
            }],
            anim_vars: vec![
                AnimVar {
                    name: "x",
                    inverse: false,
                },
                AnimVar {
                    name: "y",
                    inverse: true,
                },
            ],
        };

        assert_eq!(actual, expected);
    }
}
