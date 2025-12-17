//! A module for prioritized JSON patch operations.
//!
//! This crate is designed for patching JSON structures where each patch has a
//! priority value. Conflicting patches can be resolved based on their priority.
//!
//! `Patch::One` is used for scalar fields or single-class replacements,
//! while `Patch::Seq` is used for editing arrays, with each operation targeting
//! a specific index range.
mod apply;
pub mod json_path;
mod operation;
mod patch_types;
pub mod ptr_mut;
pub(crate) mod range;
pub(crate) mod vec_utils;

pub use self::apply::error::{JsonPatchError, Result};
pub use self::apply::{
    one_op::apply_one_field,
    seq::{apply_seq_array_directly, apply_seq_by_priority},
};
pub use self::json_path::JsonPath;
pub use self::operation::Op;
pub use self::patch_types::{Action, JsonPatch, ValueWithPriority};
pub use self::range::parse::parse_range;
