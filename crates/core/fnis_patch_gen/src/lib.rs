// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.
//! This module defines FNIS animation types and flags.
//!
//! All animation type (`<AnimType>`) and option (`<option>`) definitions
//! are **based on and quoted from** _Fore's_ **"FNIS for Modders_V6.2.pdf"(© Fore)**,
//! which is part of the FNIS (Fores New Idles in Skyrim) modding documentation.
//!
//! # Usage (from `behavior_merge`)
//! 1. [`collect::collect_all_fnis_injections`]: find each FNIS mod's list file, behavior and animations
//! 2. [`collect_borrowed_patches`]: turn them into behavior / adsf / asdsf patches + hkx conversion jobs
pub mod collect;
pub mod errors;
mod patch_gen;

// The shared modules are imported at the root, so paths read the same as in `behavior_merge`.
// e.g., `crate::config::Config`, `crate::patches::BehaviorPatchesMap`
use behavior_merge_types::{
    Config, OutPutTarget, ParserMode, PriorityMap, adsf, asdsf, config, patches, template_key,
};

pub use self::{
    errors::Error,
    patch_gen::{FNISIoJobRunner, collect_borrowed_patches},
};
