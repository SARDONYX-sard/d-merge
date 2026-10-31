//! Data exchanged between `behavior_merge` and `fnis_patch_gen`.
//!
//! # Crate graph
//! ```text
//! behavior_merge_types   <- this crate: what both sides read or produce
//!   ^         ^
//!   |    fnis_patch_gen  <- FNIS_*_List.txt -> patches (+ its own errors)
//!   |         ^
//! behavior_merge          <- Nemesis patches, applying all patches, hkx output (entry point)
//! ```
//!
//! # What belongs here
//! Only types that cross the `fnis_patch_gen` -> `behavior_merge` boundary:
//! - [`config`]: options and progress reporting (both crates read them)
//! - [`template_key`]: which template a patch targets
//! - [`patches`], [`adsf`], [`asdsf`]: the patches `fnis_patch_gen` hands to `behavior_merge`
//! - [`priority`]: mod priorities
//!
//! Anything used by only one of them (errors, parsers, file collection) stays in that crate.

pub mod adsf;
pub mod asdsf;
pub mod config;
pub mod patches;
pub mod priority;
pub mod template_key;

pub use crate::{
    config::{
        Config, DebugOptions, HackOptions, OutPutTarget, ParserMode, Status, StatusReporterFn,
    },
    priority::{PatchMaps, PriorityMap},
};
