pub mod error;
mod one_op;
mod range_op;

use self::error::Result;
use self::one_op::add::apply_add;
use self::one_op::remove::apply_remove;
use self::one_op::replace::apply_replace;
use self::range_op::apply_range;
use crate::operation::Op;
use crate::range::parse::is_range_op;
use crate::JsonPatch;
use simd_json::BorrowedValue;

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
pub fn apply_patch<'v>(json: &mut BorrowedValue<'v>, patch: JsonPatch<'v>) -> Result<()> {
    if is_range_op(&patch.path) {
        apply_range(json, patch)
    } else {
        match patch.op {
            Op::Add => apply_add(json, patch),
            Op::Remove => apply_remove(json, patch),
            Op::Replace => apply_replace(json, patch),
        }
    }
}
