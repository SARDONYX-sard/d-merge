mod add;
mod remove;
mod replace;

use self::add::apply_add;
use self::remove::apply_remove;
use self::replace::apply_replace;
use super::error::Result;
use crate::operation::Op;
use crate::{JsonPatch, JsonPatchError, JsonPath, OpRangeKind, ValueWithPriority};
use simd_json::BorrowedValue;

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
#[inline]
pub fn apply_one_field<'v>(
    json: &mut BorrowedValue<'v>,
    path: JsonPath<'v>,
    patch: ValueWithPriority<'v>,
) -> Result<()> {
    let JsonPatch { op, value } = patch.patch;
    match op {
        OpRangeKind::Pure(Op::Add) => apply_add(json, path, value),
        OpRangeKind::Pure(Op::Remove) => apply_remove(json, path),
        OpRangeKind::Pure(Op::Replace) => apply_replace(json, path, value),
        unexpected => Err(JsonPatchError::mismatch_apply_type_from(
            unexpected, &path, &value,
        )),
    }
}
