// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.
//! This module defines FNIS animation types and flags.
//!
//! All animation type (`<AnimType>`) and option (`<option>`) definitions
//! are **based on and quoted from** _Fore's_ **"FNIS for Modders_V6.2.pdf"(© Fore)**,
//! which is part of the FNIS (Fores New Idles in Skyrim) modding documentation.
mod animations;
mod inject;
mod list_parser;

mod paths;
mod types;

use crate::behaviors::tasks::fnis::list_parser::{
    anim_types::FNISAnimType, flags::FNISAnimFlagSet,
};

/// Represents a FNIS animation kind, consisting of a base animation type and optional flags.
///
/// Derived from FNIS animation definition syntax:
/// - `<AnimType>` describes the core behavior (e.g., basic, sequenced, furniture, paired).
/// - `<option>` describes modifiers (e.g., acyclic, animated camera, headtracking).
#[derive(Debug, PartialEq)]
pub struct FNISAnimKind<'a> {
    /// The main animation type (FNISAnimType).
    pub anim_type: FNISAnimType,
    /// Bitflags representing animation modifiers (FNISAnimFlags).
    pub flags: FNISAnimFlagSet<'a>,
}

impl<'a> FNISAnimKind<'a> {
    /// Creates a new FNISAnimKind with the given animation type and flags.
    #[inline]
    pub const fn new(animation_type: FNISAnimType, flags: FNISAnimFlagSet<'a>) -> Self {
        Self {
            anim_type: animation_type,
            flags,
        }
    }
}
