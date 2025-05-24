//! Parser for patch files with optional support for compatibility hacks.
//!
//! This module provides parsers for reading structured patch files, with an emphasis on flexibility
//! and robustness when handling modded or unofficial data. In particular, it includes targeted workarounds
//! for known issues in community-created patches that would otherwise fail validation.
//!
//! ## Features
//! - Standard parsing for fields and types defined in patch files
//! - Hack options to support legacy or incorrect data
//! - Specialized fixes, such as correcting `event` → `contactEvent` in `BSRagdollContactListenerModifier`
//!
//! ## Hack Behavior
//! The [`HackOptions`] structs expose flags that enable selective leniency
//! during parsing. For example, the [`do_hack_cast_ragdoll_event`] function replaces the invalid
//! `event` field in the ragdoll modifier class with a valid `contactEvent` field of type
//! `Object|hkbEventProperty`.
//!
//! These hacks are intended for compatibility with real-world data and modding ecosystems,
//! and are not recommended for validating strictly well-formed files.
//!
//! ## Safety and Limitations
//! Enabling hacks may allow invalid data to pass silently. Use only when working with trusted
//! or known input formats, and prefer strict parsing in validation or toolchain scenarios.
mod bs_ragdoll_event;

pub use bs_ragdoll_event::do_hack_cast_ragdoll_event;

/// A collection of hack options that enable non-standard parsing behavior.
///
/// These options exist to handle cases where game mods or other tools produce
/// invalid or inconsistent data. Enabling these may allow parsing to succeed
/// in otherwise broken scenarios, at the risk of hiding real errors.
#[derive(Debug, Copy, Clone, Default)]
pub struct HackOptions {
    /// Enables compatibility hacks for invalid fields in the `BSRagdollContactListenerModifier` class.
    ///
    /// This option activates targeted fixes for common field naming mistakes in patches:
    /// - Substitutes `event` with `contactEvent`
    /// - Substitutes `anotherBoneIndex` with `bones`
    pub cast_ragdoll_event: bool,
}

impl HackOptions {
    /// Enable all hack options.
    #[inline]
    pub const fn enable_all() -> Self {
        Self {
            cast_ragdoll_event: true,
        }
    }
}
