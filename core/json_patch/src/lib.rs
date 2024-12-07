mod apply;
mod operation;
pub mod ptr_mut;
pub(crate) mod range;
pub(crate) mod vec_utils;

pub use self::apply::error::{JsonPatchError, Result};
pub use self::apply::{apply_patch, JsonPatch};
pub use self::operation::Op;
