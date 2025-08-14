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
use winnow::ascii::{dec_int, digit1, float, multispace0, space0, space1, till_line_ending};
use winnow::combinator::{alt, delimited, opt, preceded, repeat, seq};
use winnow::error::{StrContext, StrContextValue};
use winnow::token::take_till;
use winnow::{ModalResult, Parser as _};

use crate::behaviors::tasks::fnis::{FNISAnimFlags, FNISAnimKind, FNISAnimType};

#[derive(Debug)]
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

#[derive(Debug)]
struct Entry<'i> {
    /// type + flags
    pub kind: FNISAnimKind,
    pub event: &'i str,
    /// `.hkx` file name
    pub file: &'i str,
    pub md: MotionData,
    pub rd: RotationData,
}

#[derive(Debug)]
pub struct MotionData {
    pub time: f32,
    pub delta_x: i64,
    pub delta_y: i64,
    pub delta_z: i64,
}

#[derive(Debug)]
pub enum RotationData {
    Format1(RotationData1),
    Format2(RotationData2),
}

#[derive(Debug)]
pub struct RotationData1 {
    pub time: f32,
    pub quat_1: i64,
    pub quat_2: i64,
    pub quat_3: i64,
    pub quat_4: i64,
}

#[derive(Debug)]
pub struct RotationData2 {
    pub time: f32,
    pub delta_z_angle: i64,
}

pub fn parse_anim_list<'i>(input: &mut &'i str) -> ModalResult<FnisAnimList<'i>> {
    line_comments0.parse_next(input)?;
    let version = parse_version_line.parse_next(input)?;

    let mut entries = vec![];
    while let Ok(entry) = parse_entry.parse_next(input) {
        entries.push(entry);
    }

    line_comments0.parse_next(input)?;

    Ok(FnisAnimList { version, entries })
}

fn parse_entry<'i>(input: &mut &'i str) -> ModalResult<Entry<'i>> {
    line_comments0.parse_next(input)?;

    let kind = parse_anim_preset.parse_next(input)?;
    space1.parse_next(input)?;
    let event = take_till(1.., (' ', '\t')).parse_next(input)?;
    space1.parse_next(input)?;
    let file = take_till(1.., '\n').parse_next(input)?;
    "\n".parse_next(input)?;

    line_comments0.parse_next(input)?;
    let md = parse_md_data.parse_next(input)?;

    line_comments0.parse_next(input)?;
    let rd = parse_rd_data.parse_next(input)?;

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
            time: float,
            _: space1,
            delta_x: dec_int,
            _: space1,
            delta_y: dec_int,
            _: space1,
            delta_z: dec_int,
            _: (opt("\r"), "\n")
        }),
    )
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
        time: float,
        _: space1,
        quat_1: dec_int,
        _: space1,
        quat_2: dec_int,
        _: space1,
        quat_3: dec_int,
        _: space1,
        quat_4: dec_int,
    })
    .parse_next(input)
}

/// `RD <time> <quat_1> <quat_2> <quat_3> <quat_4>`
fn parse_rd_data2(input: &mut &str) -> ModalResult<RotationData2> {
    seq!(RotationData2 {
        time: float,
        _: space1,
        delta_z_angle: dec_int,
    })
    .parse_next(input)
}

/// Comments starting with `'` until newline. 0 or more.
pub fn line_comments0(input: &mut &str) -> ModalResult<()> {
    /// Comment starting with `'` until newline
    fn line_comment<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
        delimited(multispace0, preceded('\'', till_line_ending), multispace0)
            .context(StrContext::Label("Comment"))
            .context(StrContext::Expected(StrContextValue::Description(
                "Comment: e.g. `' Any String`",
            )))
            .parse_next(input)
    }

    let _: () = repeat(0.., line_comment).parse_next(input)?;
    Ok(())
}

/// ```ebnf
/// version_line := 'Version' ' '    version
/// version := digit '.' digit '.' digit string
/// ```
fn parse_version_line(input: &mut &str) -> ModalResult<Version> {
    let _ = "Version".parse_next(input)?;
    multispace0.parse_next(input)?;

    winnow::seq! {
        Version{
            major: digit1.parse_to(),
            minor: opt(preceded(".", digit1.parse_to())).map(|n| n.unwrap_or(0)),
        }
    }
    .parse_next(input)
}

/// ```ebnf
/// <anim type> flags string_until_line_ending
/// <flags> : = '-' flags *
/// <flag> := flags ','
/// ```
fn parse_anim_preset(input: &mut &str) -> ModalResult<FNISAnimKind> {
    let anim_type = parse_anim_type.parse_next(input)?;

    // When `-` comes, parse the flag list.
    let anim_flags = if opt('-').parse_next(input)?.is_some() {
        parse_anim_flag.parse_next(input)?
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
    .parse_next(input)
}

/// e.g. `ac0,ac1`
fn parse_anim_flags(input: &mut &str) -> ModalResult<FNISAnimFlags> {
    let mut anim_flags = FNISAnimFlags::empty();

    loop {
        space0.parse_next(input)?;

        let flag = parse_anim_flag(input)?;
        anim_flags |= flag;

        // If the next token is a comma, consume it and continue loop
        space0.parse_next(input)?;
        if opt(',').parse_next(input)?.is_some() {
            continue;
        }
        break;
    }

    space0.parse_next(input)?;
    Ok(anim_flags)
}

fn parse_anim_flag(input: &mut &str) -> ModalResult<FNISAnimFlags> {
    alt((
        "a".value(FNISAnimFlags::Acyclic),
        "o".value(FNISAnimFlags::AnimObjects),
        "ac".value(FNISAnimFlags::AnimatedCamera),
        "ac1".value(FNISAnimFlags::AnimatedCameraSet),
        "ac0".value(FNISAnimFlags::AnimatedCameraReset),
        "bsa".value(FNISAnimFlags::BSA),
        "h".value(FNISAnimFlags::HeadTracking),
        "k".value(FNISAnimFlags::Known),
        "md".value(FNISAnimFlags::MotionDriven),
        "st".value(FNISAnimFlags::Sticky),
        "Tn".value(FNISAnimFlags::TransitionNext),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main() {
        let input = r#"Version 7.0

' Comment

s FNISfl_Start FNISfl_Start.hkx
MD 2.9333 0 0 30
RD 2.9333 0
+ FNISfl_Back FNISfl_Back.hkx
MD 1.5 0 -150 0
RD 1.5 0
"#;

        let parsed = parse_anim_list
            .parse(input)
            .unwrap_or_else(|e| panic!("{e}"));
        println!("{:#?}", parsed);
    }
}
