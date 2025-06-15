//! Why separate modules?
//! Patches need to be parsed fairly loosely; modders may not put in line breaks.
use crate::adsf::{ClipAnimDataBlock, ClipMotionBlock, Rotation, Translation};
use crate::{adsf::de::from_word_and_space, lines::Str};
use core::str::FromStr;
use serde_hkx::errors::readable::ReadableError;
use winnow::{
    ascii::{line_ending, multispace0, till_line_ending},
    combinator::opt,
    error::{ContextError, ErrMode, StrContext::*, StrContextValue::*},
    seq, ModalResult, Parser,
};

/// Parse 1 line.
fn one_line<'a>(input: &mut &'a str) -> ModalResult<Str<'a>> {
    let line = till_line_ending.parse_next(input)?;
    // In the case of patches, this may not be present, so `opt`
    opt(line_ending).parse_next(input)?; // skip line end
    Ok(line.into())
}

fn lines<'a>(read_len: usize) -> impl Parser<&'a str, Vec<Str<'a>>, ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        let mut lines = vec![];
        for _ in 0..read_len {
            lines.push(one_line.parse_next(input)?);
        }
        Ok(lines)
    }
}

/// Parse one line and then parse to T.
fn verify_line_parses_to<'a, T>(input: &mut &'a str) -> ModalResult<Str<'a>>
where
    T: FromStr,
{
    // For some reason, using parse_to for Cow causes an error, so the method chain of the existing parser is used.
    let line = till_line_ending
        .verify(|s: &str| s.parse::<T>().is_ok())
        .parse_next(input)?;
    opt(line_ending).parse_next(input)?; // skip line end
    Ok(line.into())
}

/// Parse one line and then parse to T.
fn parse_one_line<T: FromStr>(input: &mut &str) -> ModalResult<T> {
    // For some reason, using parse_to for Cow causes an error, so the method chain of the existing parser is used.
    let line = till_line_ending.parse_to().parse_next(input)?;
    // In the case of patches, this may not be present, so `opt`
    opt(line_ending).parse_next(input)?; // skip line end
    Ok(line)
}

/// Parses the animation data structure from the input.
///
/// # Errors
/// If parsing fails, returns human readable error.
#[inline]
pub fn parse_clip_anim_block_patch(input: &str) -> Result<ClipAnimDataBlock<'_>, ReadableError> {
    clip_anim_block_patch
        .parse(input)
        .map_err(|e| ReadableError::from_parse(e))
}

/// Parses `ClipAnimDataBlock`
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_anim_block_patch<'a>(input: &mut &'a str) -> ModalResult<ClipAnimDataBlock<'a>> {
    let block = seq! {ClipAnimDataBlock {
        _: multispace0,
        name: one_line.context(Expected(Description("name: str"))),
        clip_id: one_line.context(Expected(Description("clip_id: str"))),
        play_back_speed: verify_line_parses_to::<f32>.context(Expected(Description("play_back_speed: f32"))),
        crop_start_local_time: verify_line_parses_to::<f32>.context(Expected(Description("crop_start_local_time: f32"))),
        crop_end_local_time: verify_line_parses_to::<f32>.context(Expected(Description("crop_end_local_time: f32"))),
        trigger_names_len: parse_one_line.context(Expected(Description("trigger_names_len: usize"))),
        trigger_names: lines(trigger_names_len).context(Expected(Description("trigger_names: Vec<str>"))),
        _: multispace0,
    }}
    .context(Label("ClipAnimDataBlock"))
    .parse_next(input)?;
    Ok(block)
}

/// Parses `ClipMotionBlock`
///
/// # Errors
/// If parsing fails, returns human readable error.
#[inline]
pub fn parse_clip_motion_block_patch(input: &str) -> Result<ClipMotionBlock<'_>, ReadableError> {
    clip_motion_block_patch
        .parse(input)
        .map_err(|e| ReadableError::from_parse(e))
}

/// Parses a clip motion block from the input.
///
/// # Errors
/// If parsing fails, returns an error with information (context) of where the error occurred pushed to Vec
fn clip_motion_block_patch<'a>(input: &mut &'a str) -> ModalResult<ClipMotionBlock<'a>> {
    let block = seq! {ClipMotionBlock {
            _: multispace0,
            clip_id: one_line.context(Expected(Description("clip_id: str"))),
            duration: verify_line_parses_to::<f32>.context(Expected(Description("duration: f32"))),
            translation_len: parse_one_line.context(Expected(Description("translation_len: usize"))),
            translations: translations(translation_len).context(Expected(Description("translations: Vec<Translation>"))),
            rotation_len: parse_one_line.context(Expected(Description("rotation_len: usize"))),
            rotations: rotations(rotation_len).context(Expected(Description("rotations: Vec<Rotation>"))),
            _: multispace0,
        }
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
                time: from_word_and_space::<f32>.context(Expected(Description("time: f32"))),
                x: from_word_and_space::<f32>.context(Expected(Description("x: f32"))),
                y: from_word_and_space::<f32>.context(Expected(Description("y: f32"))),
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
                time: from_word_and_space::<f32>.context(Expected(Description("time: f32"))),
                x: from_word_and_space::<f32>.context(Expected(Description("x: f32"))),
                y: from_word_and_space::<f32>.context(Expected(Description("y: f32"))),
                z: from_word_and_space::<f32>.context(Expected(Description("z: f32"))),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_motion() {
        let input = "amco$0
1.33
1
1.33 0 0 0
1
1 0 0 0 1";

        let motion = parse_clip_motion_block_patch(input).unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            motion,
            ClipMotionBlock {
                clip_id: "amco$0".into(),
                duration: "1.33".into(),
                translation_len: 1,
                translations: vec![Translation {
                    time: "1.33".into(),
                    x: "0".into(),
                    y: "0".into(),
                    z: "0".into(),
                },],
                rotation_len: 1,
                rotations: vec![Rotation {
                    time: "1".into(),
                    x: "0".into(),
                    y: "0".into(),
                    z: "0".into(),
                    w: "1".into(),
                },],
            }
        );
    }
}
