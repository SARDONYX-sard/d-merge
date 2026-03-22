mod add;
mod remove;
mod replace;

use simd_json::BorrowedValue;

use self::{add::apply_add, remove::apply_remove, replace::apply_replace};
use super::error::Result;
use crate::{operation::Op, Action, JsonPatch, JsonPatchError, JsonPath, ValueWithPriority};

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
#[inline]
pub fn apply_one_field<'v>(
    json: &mut BorrowedValue<'v>,
    path: JsonPath<'v>,
    patch: ValueWithPriority<'v>,
) -> Result<(), JsonPatchError> {
    let JsonPatch { action, value } = patch.patch;

    match action {
        Action::Pure { op: Op::Add } => apply_add(json, path, value),
        Action::Pure { op: Op::Remove } => apply_remove(json, path),
        Action::Pure { op: Op::Replace } => apply_replace(json, path, value),
        unexpected => Err(JsonPatchError::mismatch_apply_type_from(
            unexpected, &path, &value,
        )),
    }
}
