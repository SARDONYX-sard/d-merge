//! Parses animation data from adsf(animationdatasinglefile.txt)
//!
//! This module provides structures and parsers for reading animation data
//! from a file formatted in a specific way. The primary structure is [`Adsf`],
//! which contains a list of projects and their corresponding animation data.
//!
//! # Example
//!
//! ```no_run
//! let input = "..."; // Input data from the animation file
//! let result = adsf().parse(input).unwrap();
//! ```

use super::{
    lines::{from_one_line, lines, num_bool_line, one_line},
    readable_err::ReadableError,
};
use core::str::FromStr;
use winnow::{
    ascii::{line_ending, space1, till_line_ending},
    error::{ContextError, StrContext::*, StrContextValue::*},
    seq,
    token::take_till,
    PResult, Parser,
};

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Represents the entire animation data structure.
///
/// This structure contains the names of the projects and a list of associated
/// animation data.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Adsf<'a> {
    /// A list of project names parsed from the input.
    pub project_names: Vec<&'a str>,

    /// A list of animation data corresponding to each project.
    pub anim_list: Vec<AnimData<'a>>,
}

/// Represents individual animation data.
///
/// This structure holds the header information for the animation and the
/// associated clip animation and motion blocks.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimData<'a> {
    /// The header containing metadata about the animation data.
    pub header: AnimDataHeader<'a>,

    /// A list of animation blocks corresponding to the clips.
    pub clip_anim_blocks: Vec<ClipAnimDataBlock<'a>>,

    /// A list of motion blocks corresponding to the clips.
    pub clip_motion_blocks: Vec<ClipMotionBlock<'a>>,
}

/// Represents the header of animation data.
///
/// This structure contains metadata related to the animation data, such as
/// the number of lines remaining, asset count, and project assets.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimDataHeader<'a> {
    /// Number of lines remaining representing `anim_data` after this line is read.
    pub line_range: usize,

    /// An integer value related to the animation (meaning may vary based on context).
    pub lead_int: i32,

    /// The length of the project assets.
    pub project_assets_len: usize,

    /// A list of project asset names.
    pub project_assets: Vec<&'a str>,

    /// Indicates whether motion data is available.
    pub has_motion_data: bool,
}

impl AnimDataHeader<'_> {
    /// Returns the number of lines consumed to read this struct.
    const fn parsed_line_len(&self) -> usize {
        3 + self.project_assets_len + 1
    }
}

/// Represents a clip animation data block.
///
/// This structure contains information about a single animation clip, such
/// as playback speed and the trigger names associated with the clip.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipAnimDataBlock<'a> {
    /// The name of the animation clip.
    pub name: &'a str,

    /// An identifier for the animation clip.
    pub clip_id: &'a str,

    /// The playback speed of the animation.
    pub play_back_speed: f32,

    /// The start time for cropping the animation.
    pub crop_start_local_time: f32,

    /// The end time for cropping the animation.
    pub crop_end_local_time: f32,

    /// The length of the trigger names.
    pub trigger_names_len: usize,

    /// A list of names that trigger the animation.
    pub trigger_names: Vec<&'a str>,
}

impl ClipAnimDataBlock<'_> {
    /// Returns the number of lines consumed to read this struct.
    const fn parsed_line_len(&self) -> usize {
        6 + self.trigger_names_len + 1 // +1 for the empty line
    }
}

/// Represents a motion block for a clip.
///
/// This structure contains information about the duration and translation
/// and rotation data for a specific motion clip.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ClipMotionBlock<'a> {
    /// An identifier for the clip associated with this motion block.
    pub clip_id: &'a str,

    /// The duration of the motion in seconds.
    pub duration: f32,

    /// The length of the translation data.
    pub translation_len: usize,

    /// A list of translation data points.
    pub translations: Vec<Translation>,

    /// The length of the rotation data.
    pub rotation_len: usize,

    /// A list of rotation data points.
    pub rotations: Vec<Rotation>,
}

impl ClipMotionBlock<'_> {
    /// Returns the number of lines consumed to read this struct.
    const fn parsed_line_len(&self) -> usize {
        3 + self.translation_len + 1 + self.rotation_len + 1 // +1 for the empty line
    }
}

/// Represents the rotation data using a quaternion,
/// where time indicates the moment of the rotation,
/// and x, y, z, w represent the quaternion components.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Rotation {
    /// The timestamp in seconds at which this rotation occurs.
    pub time: f32,

    /// The x component of the quaternion, representing the rotation axis.
    pub x: f32,

    /// The y component of the quaternion, representing the rotation axis.
    pub y: f32,

    /// The z component of the quaternion, representing the rotation axis.
    pub z: f32,

    /// The w component of the quaternion, representing the cosine of half the rotation angle.
    /// A value of `1.0` means no rotation (identity quaternion).
    pub w: f32,
}

#[derive(Debug, Default, Clone, PartialEq)]
/// Represents the translation data (movement in space),
/// where time indicates the moment of translation,
/// and x, y, z represent the movement along the respective axes.
pub struct Translation {
    /// The timestamp in seconds at which this translation occurs.
    pub time: f32,

    /// The amount of movement along the x-axis.
    pub x: f32,

    /// The amount of movement along the y-axis.
    pub y: f32,

    /// The amount of movement along the z-axis.
    pub z: f32,
}

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Parses the animation data structure from the input.
///
/// # Errors
/// If parsing fails, returns human readable error.
pub fn parse_adsf(input: &str) -> Result<Adsf<'_>, ReadableError> {
    adsf.parse(input)
        .map_err(|e| ReadableError::from_parse(e, input))
}

fn adsf<'a>(input: &mut &'a str) -> PResult<Adsf<'a>> {
    let project_names = project_names.parse_next(input)?;

    let mut anim_list = vec![];
    for _ in 0..project_names.len() {
        let anim_data = anim_data.parse_next(input)?;
        anim_list.push(anim_data);
    }

    // Return the parsed Adsf structure
    Ok(Adsf {
        project_names,
        anim_list,
    })
}

/// Parses the project names from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn project_names<'a>(input: &mut &'a str) -> PResult<Vec<&'a str>> {
    let line_len = from_one_line.parse_next(input)?;

    let mut project_names = vec![];
    for _ in 0..line_len {
        let project_name = one_line
            .context(Expected(Description("project_name")))
            .parse_next(input)?;
        project_names.push(project_name);
    }
    Ok(project_names)
}

/// Parses animation data from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn anim_data<'a>(input: &mut &'a str) -> PResult<AnimData<'a>> {
    let header = anim_header.parse_next(input)?;
    let line_range = header.line_range;

    let mut current_line_len = header.parsed_line_len();
    let mut clip_anim_blocks = vec![];
    while current_line_len < line_range {
        let clip_anim_block = clip_anim_block.parse_next(input)?;
        current_line_len += clip_anim_block.parsed_line_len();
        clip_anim_blocks.push(clip_anim_block);
    }

    let clip_motion_blocks = if header.has_motion_data {
        clip_motion_blocks.parse_next(input)?
    } else {
        vec![]
    };

    Ok(AnimData {
        header,
        clip_anim_blocks,
        clip_motion_blocks,
    })
}

/// Parses the animation data header from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn anim_header<'a>(input: &mut &'a str) -> PResult<AnimDataHeader<'a>> {
    let header = seq! {
        AnimDataHeader {
            line_range: one_line.parse_to().context(Expected(Description("anim_line_len: usize"))),
            lead_int: one_line.parse_to().context(Expected(Description("lead_int: i32"))),
            project_assets_len: one_line.parse_to().context(Expected(Description("project_assets_len: usize"))),
            project_assets: lines(project_assets_len).context(Expected(Description("project_assets: Vec<str>"))),
            has_motion_data: num_bool_line.context(Expected(Description("has_motion_data: 1 | 0"))),
        }
    }
    .context(Label("AnimDataHeader"))
    .parse_next(input)?;
    Ok(header)
}

/// Parses a clip animation data block from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_anim_block<'a>(input: &mut &'a str) -> PResult<ClipAnimDataBlock<'a>> {
    let block = seq! {ClipAnimDataBlock {
        name: one_line.context(Expected(Description("name: str"))),
        clip_id: one_line.context(Expected(Description("clip_id: str"))),
        play_back_speed: one_line.parse_to().context(Expected(Description("play_back_speed: f32"))),
        crop_start_local_time: one_line.parse_to().context(Expected(Description("crop_start_local_time: f32"))),
        crop_end_local_time: one_line.parse_to().context(Expected(Description("crop_end_local_time: f32"))),
        trigger_names_len: from_one_line.context(Expected(Description("trigger_names_len: usize"))),
        trigger_names: lines(trigger_names_len).context(Expected(Description("trigger_names: Vec<str>"))),
        _: line_ending.context(Expected(Description("empty line"))),
    }}
    .context(Label("ClipAnimDataBlock"))
    .parse_next(input)?;
    Ok(block)
}

/// Parses multiple clip motion blocks from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_blocks<'a>(input: &mut &'a str) -> PResult<Vec<ClipMotionBlock<'a>>> {
    let line_range = from_one_line.parse_next(input)?;

    let mut motion_blocks = vec![];
    let mut current_line_len = 0;
    while current_line_len < line_range {
        let clip_motion_block = clip_motion_block.parse_next(input)?;
        current_line_len += clip_motion_block.parsed_line_len();
        motion_blocks.push(clip_motion_block);
    }
    Ok(motion_blocks)
}

/// Parses a clip motion block from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_block<'a>(input: &mut &'a str) -> PResult<ClipMotionBlock<'a>> {
    let block = seq! {ClipMotionBlock {
        clip_id: one_line.context(Expected(Description("clip_id: str"))),
        duration: one_line.parse_to().context(Expected(Description("duration: f32"))),
        translation_len: from_one_line.context(Expected(Description("translation_len: usize"))),
        translations: translations(translation_len).context(Expected(Description("translations: Vec<Translation>"))),
        rotation_len: from_one_line.context(Expected(Description("rotation_len: usize"))),
        rotations: rotations(rotation_len).context(Expected(Description("rotations: Vec<Rotation>"))),
        _: line_ending.context(Expected(Description("empty line"))),
    }}
    .context(Label("ClipMotionBlock"))
    .parse_next(input)?;
    Ok(block)
}

fn translations<'a>(line_len: usize) -> impl Parser<&'a str, Vec<Translation>, ContextError> {
    move |input: &mut &'a str| {
        let mut translations = vec![];
        for _ in 0..line_len {
            let translation = seq! {Translation {
                time: from_word_and_space.context(Expected(Description("time: f32"))),
                x: from_word_and_space.context(Expected(Description("x: f32"))),
                y: from_word_and_space.context(Expected(Description("y: f32"))),
                z: till_line_ending.parse_to().context(Expected(Description("z: f32"))),
                _: line_ending,
            }}
            .context(Label("Translation"))
            .parse_next(input)?;

            translations.push(translation);
        }

        Ok(translations)
    }
}

fn rotations<'a>(line_len: usize) -> impl Parser<&'a str, Vec<Rotation>, ContextError> {
    move |input: &mut &'a str| {
        let mut rotations = vec![];
        for _ in 0..line_len {
            let rotation = seq! {Rotation {
                time: from_word_and_space.context(Expected(Description("time: f32"))),
                x: from_word_and_space.context(Expected(Description("x: f32"))),
                y: from_word_and_space.context(Expected(Description("y: f32"))),
                z: from_word_and_space.context(Expected(Description("z: f32"))),
                w: till_line_ending.parse_to().context(Expected(Description("w: f32"))),
                _: line_ending,
            }}
            .context(Label("Rotation"))
            .parse_next(input)?;

            rotations.push(rotation);
        }

        Ok(rotations)
    }
}

/// Get a string up to a space and then consume the space.
fn word_and_space<'a>(input: &mut &'a str) -> PResult<&'a str> {
    let s = take_till(0.., |c| c == ' ').parse_next(input)?;
    space1.parse_next(input)?;
    Ok(s)
}

/// Get a string up to a space and parse to T, then consume the space.
#[inline]
fn from_word_and_space<T: FromStr>(input: &mut &str) -> PResult<T> {
    word_and_space.parse_to().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parse(input: &str) {
        match parse_adsf(input) {
            Ok(res) => {
                std::fs::create_dir_all("../dummy/debug").unwrap();
                std::fs::write("../dummy/debug/adsf_debug.txt", format!("{res:#?}")).unwrap();
            }
            Err(err) => panic!("{err}"),
        }
    }

    #[test]
    fn should_parse() {
        let s = include_str!("../../../assets/templates/meshes/animationdatasinglefile.txt");
        test_parse(s);
    }
}
