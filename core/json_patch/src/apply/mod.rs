pub mod error;
mod one_op;
mod range_op;

use self::error::Result;
use self::one_op::add::apply_add;
use self::one_op::remove::apply_remove;
use self::one_op::replace::apply_replace;
use self::range_op::apply_range;
use crate::operation::Op;
use crate::{JsonPatch, JsonPath, OpRange, OpRangeKind};
use simd_json::BorrowedValue;

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
pub fn apply_patch<'v>(
    json: &mut BorrowedValue<'v>,
    path: JsonPath<'v>,
    patch: JsonPatch<'v>,
) -> Result<()> {
    let JsonPatch { op, value } = patch;

    match op {
        // Pure
        OpRangeKind::Pure(Op::Add) => apply_add(json, path, value),
        OpRangeKind::Pure(Op::Remove) => apply_remove(json, path),
        OpRangeKind::Pure(Op::Replace) => apply_replace(json, path, value),

        //  Range
        OpRangeKind::Seq(op_range) => {
            let OpRange { op, range } = op_range;
            let range = crate::range::Range::FromTo(range);

            apply_range(json, path, op, range, value)
        }
        OpRangeKind::Discrete(vec_range) => {
            // TODO:
            for op_range in vec_range {
                let OpRange { op, range } = op_range;
                let range = crate::range::Range::FromTo(range);

                apply_range(json, path.clone(), op, range, value.clone())?;
                // if let Err(err) = apply_range(json, path.clone(), op, range, value.clone()) {
                //     return Err(err);
                // }
            }
            Ok(())
        }
    }
}
