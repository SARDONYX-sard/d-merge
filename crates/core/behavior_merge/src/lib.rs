//! Entry point of the behavior merge: applies Nemesis and FNIS patches to the behavior
//! templates and writes the hkx files.
//!
//! Shared types live in `behavior_merge_types`, FNIS patch generation in `fnis_patch_gen`
//! (see the crate graph in `behavior_merge_types`).
mod behaviors;
pub mod cache_remover;
pub mod errors;
mod results;

// Imported at the root so paths read `crate::config::..`, as before the crate split.
use behavior_merge_types::config;
pub use behavior_merge_types::{
    Config, DebugOptions, HackOptions, OutPutTarget, ParserMode, PatchMaps, PriorityMap, Status,
    StatusReporterFn,
};

pub use crate::behaviors::{behavior_gen, create_bin_templates};

#[cfg(test)]
mod tests;
