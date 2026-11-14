//! What we wish were in `std`. Named after rust-analyzer's `stdx`.
//!
//! So far, for what runs on many strings and paths: no allocations in between, as the obvious way has
//! with `to_lowercase` only to compare, or chained `Path::join`; and no copies of generated text.

pub mod arena;
pub mod path;
pub mod text;
pub mod time;
