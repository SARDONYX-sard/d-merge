//! `FNIS_<mod name>_List.txt` parser
//!
//! See FNIS for Modders pdf
//!
//!
//! 5. Syntax of AnimLists(From `FNIS for Modders_V6.2.pdf` by fore)
//!
//! ```txt
//! -        FNIS Animation: <AnimType> [-<option,option,...>] <AnimEvent> <AnimFile> [<AnimObject> ...]
//! -     Behavior Variable: AnimVar <AnimVar> [ BOOL | INT32 | REAL ] <numeric_value>
//! -           Motion Data: MD <time> <delta_x> <delta_y> <delta_z>
//! - Rotation Data Format1: RD <time> <quat_1> <quat_2> <quat_3> <quat_4>
//! - Rotation Data Format2: RD <time> <delta_z_angle>
//! -        Version of mod: Version V<n>.<m>
//! -  Alternate Animations: AAprefix <3_character_mod_abbreviation>
//! -  Alternate Animations: AAset <animation_group> <number>
//! -  Alternate Animations: T <alternate_animation> <trigger1> <time1> <trigger2> <time2> ..
//! ```
use winnow::ascii::{
    dec_int, digit1, float, multispace0, space0, space1, till_line_ending, Caseless,
};
use winnow::combinator::{alt, opt, preceded, repeat, seq, terminated};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser as _};

use crate::behaviors::priority_ids::take_until_ext;
use crate::behaviors::tasks::fnis::{FNISAnimFlags, FNISAnimKind, FNISAnimType};

#[derive(Debug, PartialEq)]
pub struct FnisAnimList<'i> {
    /// mod version
    pub version: Version,
    pub entries: Vec<Entry<'i>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
}

#[derive(Debug, PartialEq)]
struct Entry<'i> {
    /// type + flags
    pub kind: FNISAnimKind,
    pub event: &'i str,
    /// `.hkx` file name
    pub file: &'i str,
    pub md: MotionData,
    pub rd: RotationData,
}

#[derive(Debug, PartialEq)]
pub struct MotionData {
    pub time: f32,
    pub delta_x: i64,
    pub delta_y: i64,
    pub delta_z: i64,
}

#[derive(Debug, PartialEq)]
pub enum RotationData {
    Format1(RotationData1),
    Format2(RotationData2),
}

#[derive(Debug, PartialEq)]
pub struct RotationData1 {
    pub time: f32,
    pub quat_1: i64,
    pub quat_2: i64,
    pub quat_3: i64,
    pub quat_4: i64,
}

#[derive(Debug, PartialEq)]
pub struct RotationData2 {
    pub time: f32,
    pub delta_z_angle: i64,
}

pub fn parse_anim_list<'i>(input: &mut &'i str) -> ModalResult<FnisAnimList<'i>> {
    line_comments0.parse_next(input)?;
    let version = parse_version_line.parse_next(input)?;
    multispace0.parse_next(input)?;

    let mut entries = vec![];
    while let Ok(entry) = parse_entry.parse_next(input) {
        entries.push(entry);
    }

    line_comments0.parse_next(input)?;

    Ok(FnisAnimList { version, entries })
}

fn parse_entry<'i>(input: &mut &'i str) -> ModalResult<Entry<'i>> {
    line_comments0.parse_next(input)?;

    let kind = parse_anim_preset
        .context(StrContext::Label("AnimPreset"))
        .context(StrContext::Expected(StrContextValue::Description(
            "AnimType & Optional flags, e.g. `s -fu,ac0`, `pa`",
        )))
        .parse_next(input)?;
    space0.parse_next(input)?;

    dbg!(*input);

    let event = take_till(0.., (' ', '\t'))
        .context(StrContext::Label("AnimEvent"))
        .context(StrContext::Expected(StrContextValue::Description(
            "Animation event name (string without spaces)",
        )))
        .parse_next(input)?;
    space0.parse_next(input)?;

    let file = terminated(take_until_ext(0.., Caseless(".hkx")), Caseless(".hkx"))
        .take()
        .context(StrContext::Label("AnimFile"))
        .context(StrContext::Expected(StrContextValue::Description(
            "Filename ending with .hkx",
        )))
        .parse_next(input)?;

    line_comments0.parse_next(input)?;

    let md = parse_md_data
        .context(StrContext::Label("MotionData"))
        .parse_next(input)?;

    line_comments0.parse_next(input)?;

    let rd = parse_rd_data
        .context(StrContext::Label("RotationData"))
        .parse_next(input)?;

    Ok(Entry {
        kind,
        event,
        file,
        md,
        rd,
    })
}

fn parse_md_data(input: &mut &str) -> ModalResult<MotionData> {
    preceded(
        "MD",
        seq!(MotionData {
            _: space1,
            time: float
                .context(StrContext::Label("Motion time"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "Float value (e.g. 1.5, 2.9333)"
                ))),
            _: space1,
            delta_x: dec_int.context(StrContext::Label("delta_x")),
            _: space1,
            delta_y: dec_int.context(StrContext::Label("delta_y")),
            _: space1,
            delta_z: dec_int.context(StrContext::Label("delta_z")),
            _: (opt("\r"), "\n")
        }),
    )
    .context(StrContext::Label("MotionData"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: MD <time: float> <dx: int> <dy: int> <dz: int>",
    )))
    .parse_next(input)
}

fn parse_rd_data(input: &mut &str) -> ModalResult<RotationData> {
    let rotation = preceded(
        ("RD", space1),
        alt((
            parse_rd_data1.map(RotationData::Format1),
            parse_rd_data2.map(RotationData::Format2),
        )),
    )
    .parse_next(input)?;
    (opt("\r"), "\n").parse_next(input)?;

    Ok(rotation)
}

/// `RD <time> <quat_1> <quat_2> <quat_3> <quat_4>`
fn parse_rd_data1(input: &mut &str) -> ModalResult<RotationData1> {
    seq!(RotationData1 {
        time: float.context(StrContext::Label("Rotation time")),
        _: space1,
        quat_1: dec_int.context(StrContext::Label("quat_1")),
        _: space1,
        quat_2: dec_int.context(StrContext::Label("quat_2")),
        _: space1,
        quat_3: dec_int.context(StrContext::Label("quat_3")),
        _: space1,
        quat_4: dec_int.context(StrContext::Label("quat_4")),
    })
    .context(StrContext::Label("RotationData1"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: RD <time: float> <q1: int> <q2: int> <q3: int> <q4: int>",
    )))
    .parse_next(input)
}

/// `RD <time: float> <delta_z_angle: int>`
fn parse_rd_data2(input: &mut &str) -> ModalResult<RotationData2> {
    seq!(RotationData2 {
        time: float,
        _: space1,
        delta_z_angle: dec_int,
    })
    .context(StrContext::Label("RotationData2"))
    .context(StrContext::Expected(StrContextValue::Description(
        "Format: RD <time: float> <delta_z_angle: int>",
    )))
    .parse_next(input)
}

/// Comments starting with `'` until newline. 0 or more.
fn line_comments0(input: &mut &str) -> ModalResult<()> {
    /// Comment starting with `'` until newline
    fn line_comment<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
        terminated(preceded('\'', till_line_ending), multispace0)
            .context(StrContext::Label("Comment"))
            .context(StrContext::Expected(StrContextValue::Description(
                "Comment: e.g. `' Any String`",
            )))
            .parse_next(input)
    }

    multispace0.parse_next(input)?;
    let _: () = repeat(0.., line_comment).parse_next(input)?;
    Ok(())
}

/// ```ebnf
/// version_line := 'Version' ' '    version
/// version := digit '.' digit '.' digit string
/// ```
fn parse_version_line(input: &mut &str) -> ModalResult<Version> {
    winnow::seq! {
        Version{
            _: "Version",
            _: space0,
            major: digit1.parse_to(),
            minor: opt(preceded(".", digit1.parse_to())).map(|n| n.unwrap_or(0)),
            _: multispace0,
        }
    }
    .parse_next(input)
}

/// ```ebnf
/// <anim_preset> := anim_type flags event? file
/// <flags> : = '-' flags *
/// <flag> := flags ','
/// <event> := string
/// <file> := string ".hkx"
/// ```
fn parse_anim_preset(input: &mut &str) -> ModalResult<FNISAnimKind> {
    let anim_type = parse_anim_type.parse_next(input)?;
    space1.parse_next(input)?;

    // When `-` comes, parse the flag list.
    let anim_flags = if opt('-').parse_next(input)?.is_some() {
        parse_anim_flags.parse_next(input)?
    } else {
        FNISAnimFlags::NONE
    };

    Ok(FNISAnimKind::new(anim_type, anim_flags))
}

fn parse_anim_type(input: &mut &str) -> ModalResult<FNISAnimType> {
    alt((
        "b".value(FNISAnimType::Basic),
        "s".value(FNISAnimType::Sequenced),
        "so".value(FNISAnimType::SequencedOptimized),
        "fu".value(FNISAnimType::Furniture),
        "fuo".value(FNISAnimType::FurnitureOptimized),
        "+".value(FNISAnimType::SequencedContinued),
        "ofa".value(FNISAnimType::OffsetArm),
        "o".value(FNISAnimType::Basic),
        "pa".value(FNISAnimType::Paired),
        "km".value(FNISAnimType::KillMove),
        "aa".value(FNISAnimType::Alternate),
        "ch".value(FNISAnimType::Chair),
    ))
    .context(StrContext::Label("AnimType"))
    .context(StrContext::Expected(StrContextValue::Description(
        "One of: b, s, so, fu, fuo, +, ofa, o, pa, km, aa, ch",
    )))
    .parse_next(input)
}

/// e.g. `ac0,ac1`
fn parse_anim_flags(input: &mut &str) -> ModalResult<FNISAnimFlags> {
    let mut anim_flags = FNISAnimFlags::empty();

    loop {
        anim_flags |= parse_anim_flag.parse_next(input)?;

        if opt(',').parse_next(input)?.is_some() {
            space0.parse_next(input)?;
            continue;
        }
        break;
    }
    Ok(anim_flags)
}

fn parse_anim_flag(input: &mut &str) -> ModalResult<FNISAnimFlags> {
    // NOTE: important match order
    // `alt` is short-circuit evaluation, meaning if a character partially matches in the order of description,
    // it does not check subsequent characters.
    // Therefore, it tests characters in descending order of length.
    alt((
        "ac0".value(FNISAnimFlags::AnimatedCameraReset),
        "ac1".value(FNISAnimFlags::AnimatedCameraSet),
        "bsa".value(FNISAnimFlags::BSA),
        //
        "ac".value(FNISAnimFlags::AnimatedCamera),
        "md".value(FNISAnimFlags::MotionDriven),
        "st".value(FNISAnimFlags::Sticky),
        "Tn".value(FNISAnimFlags::TransitionNext),
        //
        "a".value(FNISAnimFlags::Acyclic),
        "h".value(FNISAnimFlags::HeadTracking),
        "k".value(FNISAnimFlags::Known),
        "o".value(FNISAnimFlags::AnimObjects),
    ))
    .context(StrContext::Expected(StrContextValue::Description(
        "One of: ac0, ac1, ac, bsa, md, st, Tn, a, h, k, o",
    )))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use winnow::error::{ContextError, ErrMode};

    fn must_parse<'a, O>(
        mut parser: impl winnow::Parser<&'a str, O, ErrMode<ContextError>>,
        input: &'a str,
    ) -> O {
        parser
            .parse(input)
            .unwrap_or_else(|e| panic!("ERROR:\n{e}"))
    }

    fn must_fail<'a, O>(
        mut parser: impl winnow::Parser<&'a str, O, ErrMode<ContextError>>,
        input: &'a str,
    ) {
        if parser.parse(input).is_ok() {
            panic!("[Must fail!]expected parse to fail, but got OK");
        }
    }

    // ============================
    // parse_anim_type
    // ============================

    #[test]
    fn test_parse_anim_type_valid() {
        assert_eq!(must_parse(parse_anim_type, "b"), FNISAnimType::Basic);
        assert_eq!(must_parse(parse_anim_type, "fu"), FNISAnimType::Furniture);
        assert_eq!(must_parse(parse_anim_type, "pa"), FNISAnimType::Paired);
    }

    #[test]
    fn test_parse_anim_type_invalid() {
        must_fail(parse_anim_type, "xxx");
    }

    // ============================
    // parse_anim_flag / parse_anim_flags
    // ============================

    #[test]
    fn test_parse_anim_flag_single() {
        assert_eq!(must_parse(parse_anim_flag, "a"), FNISAnimFlags::Acyclic);
        assert_eq!(
            must_parse(parse_anim_flag, "ac0"),
            FNISAnimFlags::AnimatedCameraReset
        );
    }

    #[test]
    fn test_parse_anim_flags_multiple() {
        let parsed = must_parse(parse_anim_flags, "a,ac0");
        assert!(parsed.contains(FNISAnimFlags::Acyclic));
        assert!(parsed.contains(FNISAnimFlags::AnimatedCameraReset));
    }

    // ============================
    // parse_md_data
    // ============================

    #[test]
    fn test_parse_md_data_valid() {
        let parsed = must_parse(parse_md_data, "MD 2.9333 0 0 30\n");
        assert!((parsed.time - 2.9333).abs() < 1e-6);
        assert_eq!(parsed.delta_z, 30);
    }

    #[test]
    fn test_parse_md_data_invalid_time() {
        must_fail(parse_md_data, "MD abc 0 0 30\n");
    }

    // ============================
    // parse_rd_data
    // ============================

    #[test]
    fn test_parse_rd_data_format1() {
        match must_parse(parse_rd_data, "RD 1.5 0 0 0 1\n") {
            RotationData::Format1(d) => assert_eq!(d.quat_4, 1),
            RotationData::Format2(_) => panic!("Expected Format1"),
        }
    }

    #[test]
    fn test_parse_rd_data_format2() {
        match must_parse(parse_rd_data, "RD 1.5 90\n") {
            RotationData::Format2(d) => assert_eq!(d.delta_z_angle, 90),
            RotationData::Format1(_) => panic!("Expected Format2"),
        }
    }

    #[test]
    fn test_parse_rd_data_incomplete() {
        must_fail(parse_rd_data, "RD 1.5\n");
    }

    // ============================
    // parse_version_line
    // ============================

    #[test]
    fn test_parse_version_line_ok() {
        let parsed = must_parse(parse_version_line, "Version 7.2\n");
        assert_eq!(parsed.major, 7);
        assert_eq!(parsed.minor, 2);
    }

    #[test]
    fn test_parse_version_line_invalid() {
        must_fail(parse_version_line, "Ver 7.2\n");
    }

    // ============================
    // parse_entry
    // ============================

    #[test]
    fn test_parse_anim_preset() {
        let input = r#"s -h,ac0"#;
        let parsed = must_parse(parse_anim_preset, input);
        assert_eq!(
            parsed,
            FNISAnimKind::new(
                FNISAnimType::Sequenced,
                FNISAnimFlags::AnimatedCameraReset | FNISAnimFlags::HeadTracking
            )
        );
    }

    #[test]
    fn test_parse_entry_valid() {
        let input = r#"
s -h,ac0 IdleStart IdleStart.hkx
MD 1.0 0 0 0
RD 1.0 0
"#;
        let parsed = must_parse(parse_entry, input);
        assert_eq!(
            parsed.kind,
            FNISAnimKind::new(
                FNISAnimType::Sequenced,
                FNISAnimFlags::AnimatedCameraReset | FNISAnimFlags::HeadTracking
            )
        );
        assert_eq!(parsed.event, "IdleStart");
        assert_eq!(parsed.file, "IdleStart.hkx");
    }

    #[test]
    fn test_parse_entry_missing_file() {
        let input = r#"
s IdleStart
MD 1.0 0 0 0
RD 1.0 0
"#;
        must_fail(parse_entry, input);
    }

    #[test]
    fn test_parse_anim_list_multiple_entries() {
        let input = r#"
Version 1.0

s IdleStart IdleStart.hkx
MD 1.0 0 0 0
RD 1.0 0

fu -h,ac0 SitDown SitDown.hkx
MD 2.0 0 -10 0
RD 2.0 0 0 0 1
"#;

        let parsed = must_parse(parse_anim_list, input);
        assert_eq!(parsed.version.major, 1);
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(
            parsed.entries[0],
            Entry {
                kind: FNISAnimKind::new(FNISAnimType::Sequenced, FNISAnimFlags::empty()),
                event: "IdleStart",
                file: "IdleStart.hkx",
                md: MotionData {
                    time: 1.0,
                    delta_x: 0,
                    delta_y: 0,
                    delta_z: 0,
                },
                rd: RotationData::Format2(RotationData2 {
                    time: 1.0,
                    delta_z_angle: 0,
                }),
            }
        );
        assert_eq!(
            parsed.entries[1],
            Entry {
                kind: FNISAnimKind::new(
                    FNISAnimType::Furniture,
                    FNISAnimFlags::HeadTracking | FNISAnimFlags::AnimatedCameraReset
                ),
                event: "SitDown",
                file: "SitDown.hkx",
                md: MotionData {
                    time: 2.0,
                    delta_x: 0,
                    delta_y: -10,
                    delta_z: 0,
                },
                rd: RotationData::Format1(RotationData1 {
                    time: 2.0,
                    quat_1: 0,
                    quat_2: 0,
                    quat_3: 0,
                    quat_4: 1,
                }),
            }
        );
    }

    #[test]
    #[ignore = "local test"]
    fn test_parse_real_file() {
        let input = std::fs::read_to_string(
            "../../dummy/fnis_test_mods/FNIS Flyer SE 7.0/Data/Meshes/actors/character/animations/FNISFlyer/FNIS_FNISFLyer_List.txt"
        ).unwrap();
        let parsed = must_parse(parse_anim_list, &input);
        println!("{:#?}", parsed);
    }
}
