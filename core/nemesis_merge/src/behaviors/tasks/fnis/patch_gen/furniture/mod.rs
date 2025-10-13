//! ## FNIS Template Expression Syntax (`$...$`)
//!
//! To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
//!
//! This module deals with FNIS template expressions found in XMLs, such as:
//!
//! ```text
//! $-o+|#%RI+1%|h-|#%RI+1%|ac1|#%RI+1%|null$
//! ```
//!
//! ### Tentative Interpretation
//! These expressions appear to encode **conditional logic** that decides which
//! class or node reference to use in a state machine.
//! They often follow a pattern like:
//!
//! ```text
//! $<condition>|<value>|<condition>|<value>|...|<default>$
//! ```
//!
//! Where each `<condition>` likely corresponds to a flag (AnimObjects, HeadTracking,
//! AnimatedCameraSet, etc.), and `<value>` is the corresponding HKX node reference
//! or null.
//! Multiple conditions appear to be evaluated in order until one applies.
//!
//! ### Example Mapping
//!
//! | Token  | Possible Meaning | Rust equivalent (example) |
//! |--------|-----------------|---------------------------|
//! | `-o+`  | AnimObjects ON | `flags.contains(FNISAnimFlags::AnimObjects)` |
//! | `h-`   | HeadTracking OFF | `!flags.contains(FNISAnimFlags::HeadTracking)` |
//! | `ac1`  | AnimatedCameraSet ON | `flags.contains(FNISAnimFlags::AnimatedCameraSet)` |
//! | `null` | fallback / no match | `"#0000"` |
//!
//! ### Notes
//! - The leading or trailing `+`/`-` appears to indicate whether the flag should
//!   be considered ON or OFF for the condition.
//! - This is **not officially documented**; the interpretation is based on observed
//!   patterns in FNIS templates and may not be universally accurate.
mod one_anim;
pub mod one_group;
