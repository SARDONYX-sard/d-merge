pub mod merge;
mod operation;
pub mod ptr_mut;

pub use self::merge::error::{Error, Result};
pub use self::merge::{apply_patch, PatchJson};
pub use self::operation::Op;
