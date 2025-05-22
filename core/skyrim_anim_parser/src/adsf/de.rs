//! Parses animation data from adsf(animationdatasinglefile.txt)
//!
//! This module provides structures and parsers for reading animation data
//! from a file formatted in a specific way. The primary structure is [`Adsf`],
//! which contains a list of projects and their corresponding animation data.
use super::{
    Adsf, AnimData, AnimDataHeader, ClipAnimDataBlock, ClipMotionBlock, Rotation, Translation,
};
use crate::lines::{
    lines, num_bool_line, one_line, parse_one_line, txt_one_line, verify_line_parses_to, Str,
};
use core::str::FromStr;
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::{line_ending, multispace0, space1, till_line_ending},
    combinator::opt,
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    seq,
    token::take_till,
    ModalResult, Parser,
};

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Parses the animation data structure from the input.
///
/// # Errors
/// If parsing fails, returns human readable error.
pub fn parse_adsf(input: &str) -> Result<Adsf<'_>, ReadableError> {
    adsf.parse(input).map_err(|e| ReadableError::from_parse(e))
}

fn adsf<'a>(input: &mut &'a str) -> ModalResult<Adsf<'a>> {
    // DefaultMale
    // DefaultMale.txt
    let project_names = project_names
        .context(Expected(Description("project_names: *.txt")))
        .parse_next(input)?;

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
fn project_names<'a>(input: &mut &'a str) -> ModalResult<Vec<Str<'a>>> {
    let line_len = parse_one_line
        .context(Expected(Description("project_names_len: usize")))
        .parse_next(input)?;

    (move |input: &mut &'a str| {
        let mut lines = vec![];
        for _ in 0..line_len {
            lines.push(txt_one_line.parse_next(input)?);
        }
        Ok(lines)
    })
    .context(Expected(Description("project_names: Vec<Str>")))
    .parse_next(input)
}

/// Parses animation data from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn anim_data<'a>(input: &mut &'a str) -> ModalResult<AnimData<'a>> {
    let (line_range, header) = anim_header
        .context(Label("AnimDataHeader"))
        .parse_next(input)?;

    let mut current_line_len = header.to_line_len();
    let mut clip_anim_blocks = vec![];
    while current_line_len < line_range {
        let clip_anim_block = clip_anim_block.parse_next(input)?;
        current_line_len += clip_anim_block.to_line_len();
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
        ..Default::default()
    })
}

/// Parses the animation data header from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn anim_header<'a>(input: &mut &'a str) -> ModalResult<(usize, AnimDataHeader<'a>)> {
    // Number of lines when `AnimDataHeader` & `clip_anim_blocks: Vec<ClipAnimDataBlock>`(+ add) are serialized.
    let line_range = parse_one_line
        .context(Expected(Description("anim_line_len: usize")))
        .parse_next(input)?;

    let lead_int = parse_one_line
        .context(Expected(Description("lead_int: i32")))
        .parse_next(input)?;
    let project_assets_len = parse_one_line
        .context(Expected(Description("project_assets_len: usize")))
        .parse_next(input)?;
    let project_assets = lines(project_assets_len)
        .context(Expected(Description("project_assets: Vec<str>")))
        .parse_next(input)?;

    let has_motion_data = num_bool_line
        .context(Expected(Description("has_motion_data: 1 | 0")))
        .parse_next(input)?;

    Ok((
        line_range,
        AnimDataHeader {
            lead_int,
            project_assets,
            has_motion_data,
        },
    ))
}

/// Parses the animation data structure from the input.
///
/// # Errors
/// If parsing fails, returns human readable error.
pub fn parse_clip_anim_block_patch(input: &str) -> Result<ClipAnimDataBlock<'_>, ReadableError> {
    clip_anim_block_patch
        .parse(input)
        .map_err(|e| ReadableError::from_parse(e))
}

/// Parses `ClipAnimDataBlock`
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_anim_block_common<'a>(input: &mut &'a str) -> ModalResult<ClipAnimDataBlock<'a>> {
    let block = seq! {ClipAnimDataBlock {
        name: one_line.context(Expected(Description("name: str"))),
        clip_id: one_line.context(Expected(Description("clip_id: str"))),
        play_back_speed: verify_line_parses_to::<f32>.context(Expected(Description("play_back_speed: f32"))).map(|s| s.into()),
        crop_start_local_time: verify_line_parses_to::<f32>.context(Expected(Description("crop_start_local_time: f32"))).map(|s| s.into()),
        crop_end_local_time: verify_line_parses_to::<f32>.context(Expected(Description("crop_end_local_time: f32"))).map(|s| s.into()),
        trigger_names_len: parse_one_line.context(Expected(Description("trigger_names_len: usize"))),
        trigger_names: lines(trigger_names_len).context(Expected(Description("trigger_names: Vec<str>"))),
    }}
    .parse_next(input)?;
    Ok(block)
}

/// Parses `ClipAnimDataBlock`
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_anim_block<'a>(input: &mut &'a str) -> ModalResult<ClipAnimDataBlock<'a>> {
    let (block,) = seq! {
        clip_anim_block_common,
        _: line_ending.context(Expected(Description("empty line"))),
    }
    .context(Label("ClipAnimDataBlock"))
    .parse_next(input)?;
    Ok(block)
}

/// Parses `ClipAnimDataBlock`
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_anim_block_patch<'a>(input: &mut &'a str) -> ModalResult<ClipAnimDataBlock<'a>> {
    let (block,) = seq! {
        _: multispace0,
        clip_anim_block_common,
        _: multispace0,
    }
    .context(Label("ClipAnimDataBlock"))
    .parse_next(input)?;
    Ok(block)
}

/// Parses multiple clip motion blocks from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_blocks<'a>(input: &mut &'a str) -> ModalResult<Vec<ClipMotionBlock<'a>>> {
    let line_range = parse_one_line.parse_next(input)?;

    let mut motion_blocks = vec![];
    let mut current_line_len = 0;
    while current_line_len < line_range {
        let clip_motion_block = clip_motion_block.parse_next(input)?;
        current_line_len += clip_motion_block.to_line_len();
        motion_blocks.push(clip_motion_block);
    }
    Ok(motion_blocks)
}

/// Parses `ClipMotionBlock`
///
/// # Errors
/// If parsing fails, returns human readable error.
pub fn parse_clip_motion_block_patch(input: &str) -> Result<ClipMotionBlock<'_>, ReadableError> {
    clip_motion_block_patch
        .parse(input)
        .map_err(|e| ReadableError::from_parse(e))
}

/// Parses a clip motion block from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_block_common<'a>(input: &mut &'a str) -> ModalResult<ClipMotionBlock<'a>> {
    let block = seq! {ClipMotionBlock {
        clip_id: one_line.context(Expected(Description("clip_id: str"))),
        duration: verify_line_parses_to::<f32>.context(Expected(Description("duration: f32"))).map(|s| s.into()),
        translation_len: parse_one_line.context(Expected(Description("translation_len: usize"))),
        translations: translations(translation_len).context(Expected(Description("translations: Vec<Translation>"))),
        rotation_len: parse_one_line.context(Expected(Description("rotation_len: usize"))),
        rotations: rotations(rotation_len).context(Expected(Description("rotations: Vec<Rotation>"))),
    }}
    .parse_next(input)?;
    Ok(block)
}

/// Parses a clip motion block from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_block<'a>(input: &mut &'a str) -> ModalResult<ClipMotionBlock<'a>> {
    let (block,) = seq! {
        clip_motion_block_common,
        _: line_ending.context(Expected(Description("empty line"))),
    }
    .context(Label("ClipMotionBlock"))
    .parse_next(input)?;
    Ok(block)
}

/// Parses a clip motion block from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_block_patch<'a>(input: &mut &'a str) -> ModalResult<ClipMotionBlock<'a>> {
    let (block,) = seq! {
        _: multispace0,
        clip_motion_block_common,
        _: multispace0,
    }
    .context(Label("ClipMotionBlock"))
    .parse_next(input)?;
    Ok(block)
}

fn translations<'a>(
    line_len: usize,
) -> impl Parser<&'a str, Vec<Translation<'a>>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut translations = vec![];
        for _ in 0..line_len {
            let translation = seq! {Translation {
                time: from_word_and_space::<f32>.context(Expected(Description("time: f32"))).map(|s| s.into()),
                x: from_word_and_space::<f32>.context(Expected(Description("x: f32"))).map(|s| s.into()),
                y: from_word_and_space::<f32>.context(Expected(Description("y: f32"))).map(|s| s.into()),
                z: till_line_ending.verify(|s:&str| s.parse::<f32>().is_ok()).context(Expected(Description("z: f32"))).map(|s:&str| s.into()),
                _: opt(line_ending),
            }}
            .context(Label("Translation"))
            .parse_next(input)?;

            translations.push(translation);
        }

        Ok(translations)
    }
}

fn rotations<'a>(
    line_len: usize,
) -> impl Parser<&'a str, Vec<Rotation<'a>>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut rotations = vec![];
        for _ in 0..line_len {
            let rotation = seq! {Rotation {
                time: from_word_and_space::<f32>.context(Expected(Description("time: f32"))).map(|s| s.into()),
                x: from_word_and_space::<f32>.context(Expected(Description("x: f32"))).map(|s| s.into()),
                y: from_word_and_space::<f32>.context(Expected(Description("y: f32"))).map(|s| s.into()),
                z: from_word_and_space::<f32>.context(Expected(Description("z: f32"))).map(|s| s.into()),
                w: till_line_ending.verify(|s:&str| s.parse::<f32>().is_ok()).context(Expected(Description("w: f32"))).map(|s:&str| s.into()),
                _: opt(line_ending), // In the case of patches, this may not be present, so opt
            }}
            .context(Label("Rotation"))
            .parse_next(input)?;

            rotations.push(rotation);
        }

        Ok(rotations)
    }
}

/// Get a string up to a space and then consume the space.
fn word_and_space<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let s = take_till(0.., |c| c == ' ').parse_next(input)?;
    space1.parse_next(input)?;
    Ok(s)
}

/// Get a string up to a space and parse to T, then consume the space.
#[inline]
fn from_word_and_space<'a, T: FromStr>(input: &mut &'a str) -> ModalResult<&'a str> {
    word_and_space
        .verify(|s: &str| s.parse::<T>().is_ok())
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_project_names() {
        let input = r"5
            ChickenProject.txt
            HareProject.txt
            AtronachFlame.txt
            AtronachFrostProject.txt
            AtronachStormProject.txt
";

        let project_names = project_names.parse(input).unwrap_or_else(|e| panic!("{e}"));
        dbg!(project_names);
    }

    #[quick_tracing::init(level = "DEBUG", file = "./log/test.log", stdio = false)]
    fn test_parse(input: &str) {
        let adsf = parse_adsf(input).unwrap_or_else(|err| {
            panic!("Failed to parse adsf:\n{err}");
        });

        tracing::debug!("project_names = {}", adsf.project_names.len());
        tracing::debug!("anim_list ={}", adsf.anim_list.len());
        for anim_data in adsf.anim_list {
            tracing::debug!("assets_len = {}", anim_data.header.project_assets.len());
            tracing::debug!("project_assets = {:?}", anim_data.header.project_assets);
            tracing::debug!("anim_blocks = {}", anim_data.clip_anim_blocks.len());
            tracing::debug!("motion_blocks = {}\n", anim_data.clip_motion_blocks.len());
        }

        // std::fs::create_dir_all("../dummy/debug").unwrap();
        // std::fs::write("../dummy/debug/adsf_debug.txt", format!("{res:#?}")).unwrap();
    }

    #[test]
    fn should_parse() {
        let s = include_str!(
            "../../../../resource/assets/templates/meshes/animationdatasinglefile.txt"
        );
        test_parse(s);
    }

    #[test]
    #[cfg(feature = "alt_map")]
    fn should_write_alt_adsf_json() {
        use crate::adsf::AltAdsf;

        let input = include_str!(
            "../../../../resource/assets/templates/meshes/animationdatasinglefile.txt"
        );
        let adsf = parse_adsf(input).unwrap_or_else(|err| {
            panic!("Failed to parse adsf:\n{err}");
        });
        let alt_adsf: AltAdsf = adsf.into();

        std::fs::create_dir_all("../../dummy/debug/").unwrap();
        let json = serde_json::to_string_pretty(&alt_adsf).unwrap_or_else(|err| {
            panic!("Failed to serialize adsf to JSON:\n{err}");
        });
        std::fs::write("../../dummy/debug/animationdatasinglefile.json", json).unwrap();

        let bin = rmp_serde::to_vec(&alt_adsf).unwrap();
        std::fs::write("../../dummy/debug/animationdatasinglefile.bin", bin).unwrap();
    }
}
