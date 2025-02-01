mod apply;
pub mod json_path;
mod operation;
pub mod ptr_mut;
pub(crate) mod range;
pub(crate) mod vec_utils;

pub use self::apply::apply_patch;
pub use self::apply::error::{JsonPatchError, Result};
pub use self::operation::Op;

use simd_json::BorrowedValue;
use std::borrow::Cow;
use std::ops::Range;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RangeKind {
    One(Range<usize>),
    Multi(Vec<Range<usize>>),
}

impl Default for RangeKind {
    fn default() -> Self {
        Self::One(0..0)
    }
}

/// Struct representing a JSON patch operation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonPatch<'a> {
    /// The type of operation to perform (Add, Remove, Replace).
    pub op: Op,
    /// A vector representing the path in the JSON where the operation is applied.
    ///
    /// # Example values
    /// - `["4514", "hkbStateMachineStateInfo, "generator"]`
    /// - `["1", "hkRootLevelContainer, "namedVariants", "[0]"]`
    pub path: JsonPath<'a>,
    /// The value to be added or replaced in the JSON.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
    pub range: Option<RangeKind>,
}

pub type JsonPath<'a> = Vec<Cow<'a, str>>;
