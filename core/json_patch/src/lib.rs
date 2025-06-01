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

pub use self::apply::apply_patch;
pub use self::apply::error::{JsonPatchError, Result};
pub use self::json_path::JsonPath;
pub use self::operation::Op;
pub use self::patch_types::{JsonPatch, OpRange, OpRangeKind, Patch, ValueWithPriority};
