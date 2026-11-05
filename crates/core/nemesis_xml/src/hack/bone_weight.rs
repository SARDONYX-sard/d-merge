//! Pushing to an array that is **empty** in the template.
//!
//! An empty array is written on one line, so a Nemesis patch can only replace that whole line:
//! ```xml
//! <!-- MOD_CODE ~colisc~ OPEN -->
//! <hkparam name="boneWeights" numelements="2">
//!     1.0 1.0
//! </hkparam>
//! <!-- ORIGINAL -->
//! <hkparam name="boneWeights" numelements="0"></hkparam>
//! <!-- CLOSE -->
//! ```
//! This hack only handles `boneWeights` of the mod `colisc`, but the situation is general: any
//! patch that pushes to an array with `numelements="0"` looks like this. How to support it for
//! every array is undecided.
//!
//! Note: Pandora v4 reports such a patch as an error, as the empty `<hkparam>` is not valid XML
//! for its patcher.

use simd_json::borrowed::Array;
use winnow::{
    ModalResult, Parser,
    ascii::{float, multispace0},
};

use crate::helpers::{
    comment::{CommentKind, close_comment, comment_kind},
    tag::end_tag,
};

/// Matches `<hkparam name="boneWeights" numelements="0"></hkparam>` (with surrounding whitespace).
fn bone_weight_empty_hkparam(input: &mut &str) -> ModalResult<()> {
    (multispace0, "<hkparam name=\"boneWeights\" numelements=\"0\">", multispace0, "</hkparam>", multispace0)
        .void()
        .parse_next(input)
}

/// Peeks that after the empty hkparam a `MOD_CODE ~colisc~` comment follows.
pub(crate) fn bone_weight_then_mod_code(input: &mut &str) -> ModalResult<()> {
    (bone_weight_empty_hkparam, comment_kind.verify(|kind| matches!(kind, CommentKind::ModCode("colisc"))))
        .void()
        .parse_next(input)
}

/// Parses whitespace-separated floats until `</hkparam>` (consuming the tag).
pub(crate) fn parse_floats_till_end_hkparam(input: &mut &str) -> ModalResult<Array<'static>> {
    let mut values: Array = Vec::new();
    loop {
        multispace0.parse_next(input)?;
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
