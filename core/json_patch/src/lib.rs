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
pub struct OpRange {
    pub op: Op,
    pub range: Range<usize>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpRangeKind {
    // Except for array
    Pure(Op),
    // Sequence Array
    Seq(OpRange),
    // Discrete Array
    Discrete(Vec<OpRange>),
}

impl Default for OpRangeKind {
    fn default() -> Self {
        Self::Pure(Op::Add)
    }
}

/// Struct representing a JSON patch operation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct JsonPatch<'a> {
    /// The type of operation to perform (Add, Remove, Replace).
    pub op: OpRangeKind,
    /// The value to be added or replaced in the JSON.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
}

pub type JsonPath<'a> = Vec<Cow<'a, str>>;
