mod add;
pub mod error;
pub mod range;
mod remove;
mod replace;

use self::add::apply_add;
use self::error::Result;
use self::remove::apply_remove;
use self::replace::apply_replace;
use simd_json::BorrowedValue;
use std::borrow::Cow;

/// Enum representing the type of operation for the JSON patch.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    /// Add a new value to the JSON at the specified path.
    Add,
    /// Remove the value from the JSON at the specified path.
    Remove,
    /// Replace the value at the specified path with a new value.
    Replace,
}

/// Struct representing a JSON patch operation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PatchJson<'a> {
    /// The type of operation to perform (Add, Remove, Replace).
    pub op: Op,
    /// A vector representing the path in the JSON where the operation is applied.
    ///
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    /// - e.g. "$.1.hkRootLevelContainer.namedVariants[0]",
    pub path: Vec<Cow<'a, str>>,
    /// The value to be added or replaced in the JSON.
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "BorrowedValue<'a>: serde::Deserialize<'de>"))
    )]
    pub value: BorrowedValue<'a>,
}

/// Applies a JSON patch operation to a mutable reference to a JSON value.
///
/// # Errors
/// If the patch operation fails due to an invalid operation or path not found.
pub fn apply_patch<'v>(json: &mut BorrowedValue<'v>, patch: PatchJson<'v>) -> Result<()> {
    match patch.op {
        Op::Add => apply_add(json, patch),
        Op::Remove => apply_remove(json, patch),
        Op::Replace => apply_replace(json, patch),
    }
}
