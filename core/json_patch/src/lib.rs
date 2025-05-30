mod apply;
pub mod json_path;
mod operation;
mod patch_types;
pub mod ptr_mut;
pub(crate) mod range;
pub(crate) mod vec_utils;

pub use self::apply::apply_patch;
pub use self::apply::error::{JsonPatchError, Result};
pub use self::operation::Op;
pub use self::patch_types::{JsonPatch, JsonPath, OpRange, OpRangeKind, Patch, ValueWithPriority};
