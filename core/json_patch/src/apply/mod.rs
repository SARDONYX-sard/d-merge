pub mod error;
mod one_op;
mod seq;

use self::error::Result;
use self::one_op::add::apply_add;
use self::one_op::remove::apply_remove;
use self::one_op::replace::apply_replace;
use self::seq::apply_seq_by_priority;
use crate::operation::Op;
use crate::patch_types::Patch;
use crate::{JsonPatch, JsonPath, OpRangeKind};
use simd_json::BorrowedValue;

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
///
/// # Panics
pub fn apply_patch<'v>(
    json: &mut BorrowedValue<'v>,
    path: JsonPath<'v>,
    patch: Patch<'v>,
) -> Result<()> {
    match patch {
        Patch::OneField(value) => {
            let JsonPatch { op, value } = value.patch;
            match op {
                OpRangeKind::Pure(Op::Add) => apply_add(json, path, value),
                OpRangeKind::Pure(Op::Remove) => apply_remove(json, path),
                OpRangeKind::Pure(Op::Replace) => apply_replace(json, path, value),
                _unexpected => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        "mismatch apply_patch type. Expected pure. but got {_unexpected:?}"
                    );
                    Ok(())
                }
            }
        }
        Patch::Seq(items) => apply_seq_by_priority(json, path, items),
    }
}
