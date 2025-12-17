use core::ops::Range;

use json_patch::{JsonPath, ValueWithPriority};
use rayon::prelude::*;
use serde_hkx::errors::readable::ReadableError;
use winnow::ascii::{line_ending, multispace0, till_line_ending, Caseless};
use winnow::combinator::{alt, repeat};
use winnow::error::{StrContext::*, StrContextValue::*};
use winnow::{ModalResult, Parser as _};

use crate::asdsf::normal::{AnimInfo, Attack, Condition};
use crate::asdsf::patch::de::deserializer::ArrayType;
use crate::asdsf::patch::de::error::Result;
use crate::asdsf::patch::de::{attacks_to_borrowed_value, DiffPatchAnimSetData};
use crate::common_parser::lines::{
    lines, num_bool_line, one_line, parse_one_line, verify_line_parses_to, Str,
};

/// A raw Nemesis diff block captured during parsing.
///
/// This structure does not interpret the diff content.
/// It only records where (path) and what (raw text) was modified.
#[derive(Debug, Clone)]
pub(super) struct RawDiff<'a> {
    /// Parsed category.
    ///
    /// But that doesn't necessarily mean it's correct.
    /// For example, an addition at the end of a category might actually be a diff for the next category.
    pub category: ArrayType,

    /// Array end push Operation?
    pub op: Op,

    /// The index of the array being processed. Used for range.start in .patch.
    pub seq_index: Option<usize>,

    /// JsonPath at the moment the diff block starts.
    pub path: JsonPath<'a>,

    /// Raw text slice covering `OPEN` to `ORIGINAL`/`CLOSE` (inclusive).
    pub text: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Op {
    Add,
    Replace,
    Remove,
    /// ArrayPush.
    SeqPush,
}

pub(crate) fn into_patch_map(
    raw_diffs: Vec<RawDiff<'_>>,
    priority: usize,
) -> Result<DiffPatchAnimSetData<'_>> {
    let mut patches = DiffPatchAnimSetData::default();

    for mut raw in raw_diffs {
        let value = normalize_diff(&mut raw, priority)?;

        let path = raw.path;
        let is_seq_patch = !matches!(value.patch.action, json_patch::Action::Pure { .. });
        match raw.category {
            ArrayType::Trigger => patches.triggers_patches.push(value),
            ArrayType::Condition => {
                if is_seq_patch {
                    patches.conditions_patches.seq.push(value);
                } else {
                    patches.conditions_patches.one.insert(path, value);
                }
            }
            ArrayType::Attack | ArrayType::ClipName => {
                if is_seq_patch {
                    patches.attacks_patches.seq.insert(path, value);
                } else {
                    patches.attacks_patches.one.insert(path, value);
                }
            }
            ArrayType::AnimInfo => {
                if is_seq_patch {
                    patches.anim_infos_patches.seq.push(value);
                } else {
                    patches.anim_infos_patches.one.insert(path, value);
                }
            }
        }
    }

    Ok(patches)
}

fn normalize_diff<'a>(raw: &mut RawDiff<'a>, priority: usize) -> Result<ValueWithPriority<'a>> {
    let last = raw.path.last().map(|s| s.as_ref());
    let op = raw.op;

    let (range, value) = match (raw.category, op, last) {
        (ArrayType::Condition, Op::Replace, Some("variable_name")) => (
            None,
            one_line
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?
                .into(),
        ),
        (ArrayType::Condition, Op::Replace, Some("value_a" | "value_b")) => (
            None,
            parse_one_line::<i32>
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?
                .into(),
        ),
        (ArrayType::Condition, _, None) => {
            let conditions = conditions
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?;
            let range = raw.seq_index.map(|start| Range {
                start,
                end: start + conditions.len(),
            });
            (range, conditions.into())
        }

        // attack
        (ArrayType::Attack, Op::Replace, Some("attack_trigger")) => (
            None,
            attack_trigger_line
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?
                .into(),
        ),
        (ArrayType::Attack, Op::Replace, Some("is_contextual")) => (
            None,
            num_bool_line
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?
                .into(),
        ),

        // Attack array
        (ArrayType::Attack, Op::Add | Op::Replace | Op::Remove, None) => {
            let attacks = attacks
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?;
            let range = raw.seq_index.map(|start| Range {
                start,
                end: start + attacks.len(),
            });
            (range, attacks_to_borrowed_value(attacks))
        }

        // Special Attack or clip_name patch pattern
        //
        // There are three possible patterns for SeqPush after clip_name:
        // - Push the attack itself
        // - Normal push to clip_names
        // - Push diff to anim_info_len (though this is highly unlikely)
        (ArrayType::ClipName, Op::SeqPush, _) => {
            // has unknown num bool, then maybe attack array patch
            let value = if is_attack_array_patch(raw.text) {
                let attacks = attacks
                    .parse(raw.text)
                    .map_err(|e| ReadableError::from_parse(e))?;
                raw.path.clear();
                attacks_to_borrowed_value(attacks)
            } else {
                let clip_names = clip_names_lines
                    .parse(raw.text)
                    .map_err(|e| ReadableError::from_parse(e))?; // clip_names
                raw.path.pop(); // remove `clip_name` from `["attack", "[9]", "clip_names", "clip_name"]`
                clip_names.into()
            };
            (None, value)
        }

        // clip names in Attacks
        //
        // triggers
        // - path: `[]`
        //
        // clip_names
        // - path: `["[9]", "clip_names"]`
        (ArrayType::Trigger, _, _) | (ArrayType::ClipName, _, Some("clip_names")) => {
            let vec_str = till_ending_str_lines
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?;
            let range = raw.seq_index.map(|start| Range {
                start,
                end: start + vec_str.len(),
            });
            (range, vec_str.into())
        }

        // Anim Info
        // Replacing `hashed_path` may involve the following patterns if it spans more than one line:
        //
        // - 2 lines: Replacing `hashed_path` and `hashed_file_name`
        // - 3 lines: Replacing the anim_info itself
        (ArrayType::AnimInfo, Op::Replace, Some("hashed_path"))
            if raw.text.par_lines().count() == 3 =>
        {
            let anim_infos = anim_infos
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?;
            let range = raw.seq_index.map(|start| Range {
                start,
                end: start + anim_infos.len(),
            });
            (range, anim_infos.into())
        }

        (
            ArrayType::AnimInfo,
            Op::Replace,
            Some("hashed_path" | "hashed_file_name" | "ascii_extension"),
        ) => (
            None,
            verify_line_parses_to::<u32>
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?
                .into(),
        ),

        (ArrayType::AnimInfo, Op::SeqPush, None) => {
            let anim_infos = anim_infos
                .parse(raw.text)
                .map_err(|e| ReadableError::from_parse(e))?;
            let range = raw.seq_index.map(|start| Range {
                start,
                end: start + anim_infos.len(),
            });
            (range, anim_infos.into())
        }

        failure => {
            #[cfg(feature = "tracing")]
            tracing::debug!("unknown pattern: {failure:?}");
            return Err(crate::asdsf::patch::de::error::Error::NotFoundApplyTarget {
                kind: format!("unknown pattern: {failure:?}"),
            });
        }
    };

    let action = match (range, op) {
        (None, Op::SeqPush) => json_patch::Action::SeqPush,
        (None, op) => json_patch::Action::Pure {
            op: to_json_patch_op(op),
        },
        (Some(range), op) => json_patch::Action::Seq {
            op: to_json_patch_op(op),
            range,
        },
    };

    Ok(ValueWithPriority {
        patch: json_patch::JsonPatch { action, value },
        priority,
    })
}

/// # Panics
/// `SeqPush`
const fn to_json_patch_op(op: Op) -> json_patch::Op {
    match op {
        Op::Add => json_patch::Op::Add,
        Op::Replace => json_patch::Op::Replace,
        Op::Remove => json_patch::Op::Remove,
        Op::SeqPush => unreachable!(),
    }
}

fn conditions<'a>(input: &mut &'a str) -> ModalResult<Vec<Condition<'a>>> {
    repeat(
        1..,
        winnow::seq! {
            Condition {
                _: multispace0,
                variable_name: one_line.context(Expected(Description("variable_name: str"))),
                _: multispace0,
                value_a: parse_one_line.context(Expected(Description("value_a: i32"))),
                _: multispace0,
                value_b: parse_one_line.context(Expected(Description("value_b: i32"))),
                _: multispace0,
            }
        },
    )
    .context(Label("Conditions"))
    .parse_next(input)
}

fn is_attack_array_patch<'a>(input: &'a str) -> bool {
    let result: ModalResult<(&'a str, _)> = winnow::seq! {
        _: multispace0,
        attack_trigger_line.context(Expected(Description("attack_trigger: str"))),
        _: multispace0,
        num_bool_line.context(Expected(Description("unknown: 0 | 1"))),
    }
    .context(Label("Attack"))
    .parse_peek(input);
    result.is_ok()
}

fn attacks<'a>(input: &mut &'a str) -> ModalResult<Vec<Attack<'a>>> {
    repeat(1..,winnow::seq! {
        Attack {
            _: multispace0,
            attack_trigger: one_line.context(Expected(Description("attack_trigger: str"))),
            _: multispace0,
            is_contextual: num_bool_line.context(Expected(Description("unknown: 0 | 1"))),
            _: multispace0,
            clip_names_len: parse_one_line.context(Expected(Description("clip_names_len: usize"))),
            _: multispace0,
            clip_names: lines(clip_names_len).context(Expected(Description("clip_names: Vec<str>"))),
            _: multispace0,
        }
    }.context(Label("Attack")))
    .parse_next(input)
}

fn anim_infos<'a>(input: &mut &'a str) -> ModalResult<Vec<AnimInfo<'a>>> {
    repeat(1..,winnow::seq! {
        AnimInfo {
            _: multispace0,
            hashed_path: u32_or_crc32_macro_line.context(Expected(Description("hashed_path: u32 | $crc32[<path>]$"))),
            _: multispace0,
            hashed_file_name: u32_or_crc32_macro_line.context(Expected(Description("hashed_file_name: u32 | $crc32[<path>]$"))),
            _: multispace0,
            ascii_extension: verify_line_parses_to::<u32>.context(Expected(Description("ascii_extension: u32"))),
            _: multispace0,
        }
    }.context(Label("AnimInfo")))
    .parse_next(input)
}

fn till_ending_str_lines<'a>(input: &mut &'a str) -> ModalResult<Vec<&'a str>> {
    /// Parse 1 line.
    fn one_line<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
        let line = till_line_ending.parse_next(input)?;
        line_ending.parse_next(input)?; // skip line end
        Ok(line)
    }
    repeat(1.., one_line).parse_next(input)
}

fn clip_names_lines<'a>(input: &mut &'a str) -> ModalResult<Vec<&'a str>> {
    fn is_attack_starts(s: &str) -> bool {
        fn starts_with_ignore_ascii(s: &str, prefix: &str) -> bool {
            s.len() >= prefix.len()
                && s.get(..prefix.len())
                    .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
        }

        starts_with_ignore_ascii(s, "attackStart")
            || starts_with_ignore_ascii(s, "attackPowerStart")
            || starts_with_ignore_ascii(s, "bashStart")
            || starts_with_ignore_ascii(s, "bashPowerStart")
    }

    /// Parse 1 line.
    fn one_line<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
        let line = till_line_ending.parse_next(input)?;
        line_ending.parse_next(input)?; // skip line end
        Ok(line)
    }
    repeat(1.., one_line.verify(|line| !is_attack_starts(line))).parse_next(input)
}

/// Parse attack trigger line starting with `attackStart`, `attackPowerStart`, `bashStart` or `bashPowerStart`.
fn attack_trigger_line<'i>(input: &mut &'i str) -> winnow::ModalResult<&'i str> {
    alt((
        Caseless("attackStart"),
        Caseless("attackPowerStart"),
        Caseless("bashStart"),
        Caseless("bashPowerStart"),
    ))
    .parse_next(input)?;

    till_line_ending.parse_next(input)
}

fn u32_or_crc32_macro_line<'a>(input: &mut &'a str) -> ModalResult<Str<'a>> {
    alt((
        verify_line_parses_to::<u32>,
        crc32_macro.map(|path| {
            let crc = skyrim_crc::calc_crc32_from_bytes(path.as_bytes());
            #[cfg(feature = "tracing")]
            tracing::info!("path to crc32 hash: {path} => {crc}");

            Str::Owned(format!("{crc}"))
        }),
    ))
    .parse_next(input)
}

/// # Errors
/// If not found `$crc32[` `]`
fn crc32_macro<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    winnow::combinator::delimited(
        Caseless("$crc32["),
        winnow::token::take_until(1.., "]$"),
        "]$",
    )
    .context(Expected(Description("crc32(e.g. `$crc32[sampleName]$`)")))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32_macro() {
        let path = crc32_macro.parse_next(&mut "$crc32[some/path\\path]$");
        assert_eq!(path, Ok("some/path\\path"));
        let event_name = crc32_macro.parse_next(&mut "$crc32[some/path\\path]$remain");
        assert_eq!(event_name, Ok("some/path\\path"));
    }
}
