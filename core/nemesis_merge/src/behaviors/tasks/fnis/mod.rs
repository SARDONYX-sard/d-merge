// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Pandora Behaviour Engine Contributors
//
// This is based on the logic of Pandora-Behaviour-Engine-Plus.
//! This module defines FNIS animation types and flags.
//!
//! All animation type (`<AnimType>`) and option (`<option>`) definitions
//! are **based on and quoted from** _Fore's_ **"FNIS for Modders_V6.2.pdf"(© Fore)**,
//! which is part of the FNIS (Fores New Idles in Skyrim) modding documentation.
pub mod collect;
mod list_parser;
pub mod patch_gen;

mod behavior_table_gen;
