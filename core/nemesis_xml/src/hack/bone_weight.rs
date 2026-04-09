use simd_json::borrowed::Array;
use winnow::{
    ascii::{float, multispace0},
    ModalResult, Parser,
};

use crate::helpers::{
    comment::{close_comment, comment_kind, CommentKind},
    tag::end_tag,
};

/// Matches `<hkparam name="boneWeights" numelements="0"></hkparam>` (with surrounding whitespace).
fn bone_weight_empty_hkparam(input: &mut &str) -> ModalResult<()> {
    (
        multispace0,
        "<hkparam name=\"boneWeights\" numelements=\"0\">",
        multispace0,
        "</hkparam>",
        multispace0,
    )
        .void()
        .parse_next(input)
}

/// Peeks that after the empty hkparam a `MOD_CODE ~colisc~` comment follows.
pub fn bone_weight_then_mod_code(input: &mut &str) -> ModalResult<()> {
    (
        bone_weight_empty_hkparam,
        // NOTE: `colisc` is `Precision Creatures` id
        comment_kind.verify(|kind| matches!(kind, CommentKind::ModCode("colisc"))),
    )
        .void()
        .parse_next(input)
    // Note: validate it's actually ModCode in the caller if needed
}

/// Parses whitespace-separated floats until `</hkparam>` (consuming the tag).
pub fn parse_floats_till_end_hkparam(input: &mut &str) -> ModalResult<Array<'static>> {
    let mut values: Array = Vec::new();
    loop {
        multispace0.parse_next(input)?;
        // Peek for </hkparam>
        if end_tag("hkparam").parse_peek(input).is_ok() {
            end_tag("hkparam").parse_next(input)?;
            close_comment.parse_next(input)?;
            break;
        }
        if input.is_empty() {
            break;
        }
        let v: f64 = float.parse_next(input)?;
        values.push(v.into());
    }
    Ok(values)
}
