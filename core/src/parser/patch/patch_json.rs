use std::borrow::Cow;

use simd_json::BorrowedValue;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Remove,
    Replace,
    // Move,
    // Copy,
    // Test,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatchJson<'a> {
    pub op: Op,
    /// $(root), index, className, fieldName
    /// - e.g. "$.4514.hkbStateMachineStateInfo.generator",
    pub path: Vec<Cow<'a, str>>,
    /// patch target json value
    pub value: BorrowedValue<'a>,
}
