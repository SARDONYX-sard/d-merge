use winnow::{
    Parser as _,
    ascii::multispace0,
    combinator::eof,
    error::{StrContext::*, StrContextValue::*},
};
use winnow_ext::ReadableError;

use crate::adsf::normal::ClipAnimDataBlock;

use super::common::{Error, PatchDeserializer, one_line, parse_one_line, verify_line_parses_to};

/// Parses an animation data block patch.
///
/// # Errors
///
/// Returns a [`ReadableError`] if parsing fails.
///
/// In strict mode, the declared `trigger_names_len` must match the number
/// of non-empty trigger-name lines present in the input.
///
/// In lenient mode, the declared length is ignored and all consecutive
/// non-empty trigger-name lines are read until an empty line or end of input.
#[inline]
pub fn parse_clip_anim_block_patch<const STRICT: bool>(
    input: &str,
) -> Result<ClipAnimDataBlock<'_>, ReadableError> {
    let mut de = PatchDeserializer::from_str(input);
    let result = clip_anim_block_patch::<STRICT>(&mut de);
    result.map_err(|err| de.finish_error(err))
}

/// Parses a [`ClipAnimDataBlock`] from a patch.
///
/// # Errors
///
/// Returns an error when the input cannot be parsed or when strict mode
/// detects an invalid `trigger_names_len`.
fn clip_anim_block_patch<'a, const STRICT: bool>(
    de: &mut PatchDeserializer<'a>,
) -> Result<ClipAnimDataBlock<'a>, Error> {
    let _ = de.parse_next(multispace0)?;

    let name = de.parse_next(one_line.context(Expected(Description("name: str"))))?;

    let clip_id = de.parse_next(one_line.context(Expected(Description("clip_id: str"))))?;

    let play_back_speed = de.parse_next(
        verify_line_parses_to::<f32>.context(Expected(Description("play_back_speed: f32"))),
    )?;

    let crop_start_local_time = de.parse_next(
        verify_line_parses_to::<f32>.context(Expected(Description("crop_start_local_time: f32"))),
    )?;

    let crop_end_local_time = de.parse_next(
        verify_line_parses_to::<f32>.context(Expected(Description("crop_end_local_time: f32"))),
    )?;

    let trigger_names_len =
        de.parse_next(parse_one_line.context(Expected(Description("trigger_names_len: usize"))))?;

    let trigger_names = if STRICT {
        let trigger_names = de.read_non_empty_lines("trigger_names", trigger_names_len)?;
        de.parse_next(multispace0)?;
        de.parse_next(eof).map_err(|_| Error::TooManyEntries {
            field: "trigger_names",
            expected: trigger_names_len,
        })?;
        trigger_names
    } else {
        de.read_non_empty_lines_until_end()?
    };

    de.parse_next(multispace0)?;

    Ok(ClipAnimDataBlock {
        name,
        clip_id,
        play_back_speed,
        crop_start_local_time,
        crop_end_local_time,
        trigger_names_len: if STRICT { trigger_names_len } else { trigger_names.len() },
        trigger_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid() {
        let input = "\
name
amco$0
1.33
0.0
1.33
2
event_a
event_b
";

        let block =
            parse_clip_anim_block_patch::<true>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.name, "name");
        assert_eq!(block.clip_id, "amco$0");
        assert_eq!(block.play_back_speed, "1.33");
        assert_eq!(block.crop_start_local_time, "0.0");
        assert_eq!(block.crop_end_local_time, "1.33");
        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }

    // -------------------------------------------------------------------------
    // Strict mode
    // -------------------------------------------------------------------------

    #[test]
    fn test_strict_trigger_names_exact_length() {
        let input = "\
name
amco$0
1.33
0.0
1.33
2
event_a
event_b
";

        let block =
            parse_clip_anim_block_patch::<true>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }

    #[test]
    fn test_strict_trigger_names_fewer_than_declared() {
        let input = "\
name
amco$0
1.33
0.0
1.33
2
event_a
";

        let err = parse_clip_anim_block_patch::<true>(input)
            .expect_err("strict mode must reject fewer trigger names");

        let message = err.to_string();

        assert!(message.contains("expected 2 trigger_names"), "unexpected error: {message}");
    }

    #[test]
    fn test_strict_trigger_names_more_than_declared() {
        let input = "\
name
amco$0
1.33
0.0
1.33
2
event_a
event_b
event_c
";

        let err = parse_clip_anim_block_patch::<true>(input)
            .expect_err("strict mode must reject more trigger names");

        let message = err.to_string();

        assert!(
            message.contains(" too many trigger_names: expected 2"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn test_strict_trigger_names_empty_line() {
        let input = "\
name
amco$0
1.33
0.0
1.33
2
event_a

";

        let err = parse_clip_anim_block_patch::<true>(input)
            .expect_err("strict mode must reject an empty trigger name line");

        let message = err.to_string();

        assert!(message.contains("expected 2 trigger_names"), "unexpected error: {message}");
        assert!(message.contains("got 1"), "unexpected error: {message}");
    }

    #[test]
    fn test_strict_trigger_names_zero_length() {
        let input = "\
name
amco$0
1.33
0.0
1.33
0
";

        let block =
            parse_clip_anim_block_patch::<true>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 0);
        assert!(block.trigger_names.is_empty());
    }

    // -------------------------------------------------------------------------
    // Lenient mode
    // -------------------------------------------------------------------------

    #[test]
    fn test_lenient_trigger_names_exact_length() {
        let input = "\
name
amco$0
1.33
0.0
1.33
2
event_a
event_b
";

        let block =
            parse_clip_anim_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }

    #[test]
    fn test_lenient_trigger_names_fewer_than_declared() {
        let input = "\
name
amco$0
1.33
0.0
1.33
3
event_a
event_b
";

        let block =
            parse_clip_anim_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }

    #[test]
    fn test_lenient_trigger_names_more_than_declared() {
        let input = "\
name
amco$0
1.33
0.0
1.33
1
event_a
event_b
event_c
";

        let block =
            parse_clip_anim_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 3);
        assert_eq!(block.trigger_names, ["event_a", "event_b", "event_c",]);
    }

    #[test]
    fn test_lenient_trigger_names_stops_at_empty_line() {
        let input = "\
name
amco$0
1.33
0.0
1.33
1
event_a
event_b

";

        let block =
            parse_clip_anim_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }

    #[test]
    fn test_lenient_trigger_names_stops_at_eof() {
        let input = "\
name
amco$0
1.33
0.0
1.33
3
event_a
event_b";

        let block =
            parse_clip_anim_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }

    #[test]
    fn test_lenient_trigger_names_zero_declared() {
        let input = "\
name
amco$0
1.33
0.0
1.33
0
event_a
event_b
";

        let block =
            parse_clip_anim_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(block.trigger_names_len, 2);
        assert_eq!(block.trigger_names, ["event_a", "event_b"]);
    }
}
