pub mod error;
mod one_op;
mod seq;

use self::error::Result;
use self::one_op::apply_one_field;
use self::seq::apply_seq_by_priority;
use crate::{JsonPath, Patch};
use simd_json::BorrowedValue;

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
///
/// # Panics
#[inline]
pub fn apply_patch<'v>(
    json: &mut BorrowedValue<'v>,
    path: JsonPath<'v>,
    patch: Patch<'v>,
) -> Result<()> {
    match patch {
        Patch::OneField(patch) => apply_one_field(json, path, patch),
        Patch::Seq(patches) => apply_seq_by_priority(json, path, patches),
    }
}
