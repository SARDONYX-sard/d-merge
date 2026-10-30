pub mod deserializer;
pub mod diff;
pub mod error;

pub(crate) mod line_parsers;
mod raw_diff;

pub use diff::*;
