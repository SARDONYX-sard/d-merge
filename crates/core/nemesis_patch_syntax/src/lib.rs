//! What every Nemesis patch parser shares, whatever the patch is made of.
//!
//! A patch is the data with changed blocks marked by comments:
//! ```text
//! 3
//! <!-- MOD_CODE ~mymod~ OPEN -->
//! 2
//! <!-- ORIGINAL -->
//! 1
//! <!-- CLOSE -->
//! ```
//!
//! - [`comment`]: the parsers of those comments
//! - [`cursor`]: where a parser is in a patch, and its errors with that place

pub mod comment;
pub mod cursor;
