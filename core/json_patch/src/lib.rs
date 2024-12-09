mod apply;
mod operation;
pub mod ptr_mut;
pub(crate) mod range;
pub(crate) mod vec_utils;

pub use self::apply::apply_patch;
pub use self::apply::error::{JsonPatchError, Result};
pub use self::operation::Op;

use simd_json::BorrowedValue;
use std::borrow::Cow;

/// Struct representing a JSON patch operation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct JsonPatch<'a> {
    /// The type of operation to perform (Add, Remove, Replace).
    pub op: Op,
    /// A vector representing the path in the JSON where the operation is applied.
    ///
    /// # Example values
    /// - `["4514", "hkbStateMachineStateInfo, "generator"]`
    /// - `["1", "hkRootLevelContainer, "namedVariants", "[0]"]`
    pub path: Vec<Cow<'a, str>>,
    /// The value to be added or replaced in the JSON.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
}
