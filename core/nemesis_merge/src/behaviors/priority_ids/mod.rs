//! Utilities for extracting mod identifiers from paths pointing to `Nemesis_Engine` folders.
//!
//! This module includes functionality to extract a unique mod code path from
//! a file path, and convert multiple such paths into a priority map indexed by
//! their order in the input list. It's primarily designed to work with paths
//! from modding tools or engines like Nemesis for Skyrim SE.
//!
//! # Features
//! - Parallel extraction of mod codes from paths using Rayon
//! - Custom parsing logic with detailed error reporting using `winnow`
//! - Friendly, readable error reporting via `ReadableError`
mod fnis;
mod nemesis;
pub(super) mod types;

pub(super) use self::{fnis::parse_fnis_list_path, nemesis::get_nemesis_id};
