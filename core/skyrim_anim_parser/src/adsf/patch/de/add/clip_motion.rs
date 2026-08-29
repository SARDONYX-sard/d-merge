use winnow::{
    Parser as _,
    ascii::{multispace0, till_line_ending},
    combinator::{eof, opt},
    error::{StrContext::*, StrContextValue::*},
    seq,
};
use winnow_ext::ReadableError;

use crate::adsf::normal::{ClipMotionBlock, Rotation, Translation, de::from_word_and_space};

use super::common::{Error, PatchDeserializer, parse_one_line};

/// Parses a `ClipMotionBlock` patch.
///
/// # Errors
///
/// Returns a [`ReadableError`] if parsing fails.
///
/// In strict mode, the declared `rotation_len` must match the number of
/// rotation entries present in the input.
///
/// In lenient mode, all consecutive non-empty rotation entries are read
/// until an empty line or end of input, and `rotation_len` is replaced with
/// the number of entries actually read.
#[inline]
pub fn parse_clip_motion_block_patch<const STRICT: bool>(
    input: &str,
) -> Result<ClipMotionBlock<'_>, ReadableError> {
    let mut de = PatchDeserializer::from_str(input);
    clip_motion_block_patch::<STRICT>(&mut de).map_err(|err| de.finish_error(err))
}

fn clip_motion_block_patch<'a, const STRICT: bool>(
    de: &mut PatchDeserializer<'a>,
) -> Result<ClipMotionBlock<'a>, Error> {
    de.parse_next(multispace0)?;

    let clip_id =
        de.parse_next(super::common::one_line.context(Expected(Description("clip_id: str"))))?;

    let duration = de.parse_next(
        super::common::verify_line_parses_to::<f32>.context(Expected(Description("duration: f32"))),
    )?;

    let translation_len =
        de.parse_next(parse_one_line.context(Expected(Description("translation_len: usize"))))?;

    let translations = translations(de, translation_len)?;

    let rotation_len =
        de.parse_next(parse_one_line.context(Expected(Description("rotation_len: usize"))))?;

    let rotations =
        if STRICT { read_rotations_strict(de, rotation_len)? } else { read_rotations_lenient(de)? };

    de.parse_next(multispace0)?;

    Ok(ClipMotionBlock {
        clip_id,
        duration,
        translation_len,
        translations,
        rotation_len: if STRICT { rotation_len } else { rotations.len() },
        rotations,
    })
}

fn translations<'a>(
    de: &mut PatchDeserializer<'a>,
    line_len: usize,
) -> Result<Vec<Translation<'a>>, Error> {
    let mut translations = Vec::with_capacity(line_len);

    for _ in 0..line_len {
        let translation = de.parse_next(
            seq! {
                Translation {
                    time: from_word_and_space::<f32>
                        .context(Expected(Description("time: f32"))),
                    text: till_line_ending
                        .context(Expected(Description("text: str")))
                        .map(|s: &str| s.into()),
                    _: opt(winnow::ascii::line_ending),
                }
            }
            .context(Label("Translation")),
        )?;

        translations.push(translation);
    }

    Ok(translations)
}

fn read_rotations_strict<'a>(
    de: &mut PatchDeserializer<'a>,
    expected: usize,
) -> Result<Vec<Rotation<'a>>, Error> {
    let mut rotations = Vec::with_capacity(expected);

    while rotations.len() < expected {
        if de.input.is_empty() {
            return Err(Error::UnexpectedEnd {
                field: "rotations",
                expected,
                actual: rotations.len(),
            });
        }

        let rotation = de.parse_next(rotation_line)?;

        if rotation.time.is_empty() {
            return Err(Error::InvalidLength {
                field: "rotations",
                expected,
                actual: rotations.len(),
            });
        }

        rotations.push(rotation);
    }

    de.parse_next(multispace0)?;
    de.parse_next(eof).map_err(|_| Error::TooManyEntries { field: "rotations", expected })?;

    Ok(rotations)
}

fn read_rotations_lenient<'a>(de: &mut PatchDeserializer<'a>) -> Result<Vec<Rotation<'a>>, Error> {
    let mut rotations = Vec::new();

    loop {
        if de.input.is_empty() {
            break;
        }
        match de.parse_peek(opt(till_line_ending))? {
            Some(line) => {
                if line.is_empty() {
                    break;
                }
            }
            None => break,
        };

        let rotation = de.parse_next(rotation_line)?;
        rotations.push(rotation);
    }

    Ok(rotations)
}

fn rotation_line<'a>(input: &mut &'a str) -> winnow::ModalResult<Rotation<'a>> {
    seq! {
        Rotation {
            time: from_word_and_space::<f32>.context(Expected(Description("time: f32"))),
            text: till_line_ending.context(Expected(Description("text: str"))).map(|s: &str| s.into()),
            _: opt(winnow::ascii::line_ending),
        }
    }
    .context(Label("Rotation"))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Existing regression test
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_motion() {
        let input = "aaaa$0
1.33
1
1.33 0 0 0
1
1 0 0 0 1";

        let motion =
            parse_clip_motion_block_patch::<true>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            motion,
            ClipMotionBlock {
                clip_id: "aaaa$0".into(),
                duration: "1.33".into(),
                translation_len: 1,
                translations: vec![Translation { time: "1.33".into(), text: "0 0 0".into() }],
                rotation_len: 1,
                rotations: vec![Rotation { time: "1".into(), text: "0 0 0 1".into() }],
            }
        );
    }

    // -------------------------------------------------------------------------
    // Strict mode
    // -------------------------------------------------------------------------

    #[test]
    fn test_strict_rotations_exact_length() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
2
1 0 0 0 1
2 0 0 1 0
";

        let motion =
            parse_clip_motion_block_patch::<true>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 2);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
            ]
        );
    }

    #[test]
    fn test_strict_rotations_fewer_than_declared() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
2
1 0 0 0 1
";

        let err = parse_clip_motion_block_patch::<true>(input)
            .expect_err("strict mode must reject fewer rotations");

        let message = err.to_string();

        assert!(
            message.contains("expected 2 rotations, but reached end of input after 1"),
            "unexpected error: {message}"
        );
    }

    #[test]
    fn test_strict_rotations_more_than_declared() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
1
1 0 0 0 1
2 0 0 1 0
3 0 1 0 0
";

        let err = parse_clip_motion_block_patch::<true>(input)
            .expect_err("strict mode must reject more rotations");

        let message = err.to_string();

        assert!(message.contains("too many rotations: expected 1"), "unexpected error: {message}");
    }

    #[test]
    fn test_strict_rotations_zero_length() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
0
";

        let motion =
            parse_clip_motion_block_patch::<true>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 0);
        assert!(motion.rotations.is_empty());
    }

    // -------------------------------------------------------------------------
    // Lenient mode
    // -------------------------------------------------------------------------

    #[test]
    fn test_lenient_rotations_exact_length() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
2
1 0 0 0 1
2 0 0 1 0
";

        let motion =
            parse_clip_motion_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 2);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
            ]
        );
    }

    #[test]
    fn test_lenient_rotations_fewer_than_declared() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
3
1 0 0 0 1
2 0 0 1 0
";

        let motion =
            parse_clip_motion_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 2);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
            ]
        );
    }

    #[test]
    fn test_lenient_rotations_more_than_declared() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
1
1 0 0 0 1
2 0 0 1 0
3 0 1 0 0
";

        let motion =
            parse_clip_motion_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 3);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
                Rotation { time: "3".into(), text: "0 1 0 0".into() },
            ]
        );
    }

    #[test]
    fn test_lenient_rotations_stops_at_empty_line() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
1
1 0 0 0 1
2 0 0 1 0

";

        let motion =
            parse_clip_motion_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 2);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
            ]
        );
    }

    #[test]
    fn test_lenient_rotations_stops_at_eof() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
3
1 0 0 0 1
2 0 0 1 0";

        let motion =
            parse_clip_motion_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 2);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
            ]
        );
    }

    #[test]
    fn test_lenient_rotations_zero_declared() {
        let input = "\
aaaa$0
1.33
1
1.33 0 0 0
0
1 0 0 0 1
2 0 0 1 0
";

        let motion =
            parse_clip_motion_block_patch::<false>(input).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(motion.rotation_len, 2);
        assert_eq!(
            motion.rotations,
            vec![
                Rotation { time: "1".into(), text: "0 0 0 1".into() },
                Rotation { time: "2".into(), text: "0 0 1 0".into() },
            ]
        );
    }
}
