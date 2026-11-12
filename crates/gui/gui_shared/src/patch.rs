//! Application-level state types.
//!
//! This module contains lightweight, cloneable types that represent the
//! *current phase* of long-running background operations (fetching mod info,
//! running a patch) as well as the user-facing execution mode setting.
//!
//! # Design notes
//! - All types here are pure data — no egui, no I/O.
//! - [`FetchState`] is written by the background fetch thread and read by the
//!   UI thread via `Arc<RwLock<FetchState>>`. The UI always uses `try_read` to
//!   avoid blocking the frame loop.
//! - [`DataMode`] is persisted as part of [`AppSettings`] via serde.

use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::i18n::I18nKey;

/// Values of [`PatchProgress::phase`].
///
/// Progress phases are numbered in execution order, so they are also used as `[n/STEPS]`.
pub mod phase {
    pub const IDLE: u8 = 0;
    pub const GENERATING_FNIS_PATCHES: u8 = 1;
    pub const READING_PATCHES: u8 = 2;
    pub const PARSING_PATCHES: u8 = 3;
    pub const APPLYING_PATCHES: u8 = 4;
    pub const GENERATING_HKX_FILES: u8 = 5;
    pub const DONE: u8 = 6;
    pub const ERROR: u8 = 7;

    /// Number of steps shown as `[n/STEPS]`.
    pub(crate) const STEPS: u8 = DONE;
}

#[derive(Debug, Default)]
pub struct PatchProgress {
    /// Current phase. See [`phase`].
    pub phase: AtomicU8,

    /// Current item index within the phase.
    pub index: AtomicUsize,

    /// Total item count within the phase.
    pub total: AtomicUsize,

    /// Terminal error state.
    ///
    /// Rarely written/read, so a lock is acceptable.
    pub error: parking_lot::RwLock<Option<String>>,
}

impl PatchProgress {
    /// Resets the entire patch-progress state back to idle.
    ///
    /// Must be called before starting a new patch-generation task.
    ///
    /// Clears:
    /// - phase
    /// - index
    /// - total
    /// - done flag
    /// - terminal error
    ///
    /// This operation is intentionally lock-free except for the rare
    /// `error` field.
    #[inline]
    pub fn clear(&self) {
        self.phase.store(phase::IDLE, Ordering::Relaxed);
        self.index.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        *self.error.write() = None;
    }

    /// Returns a lightweight snapshot of the current progress state.
    ///
    /// Intended for UI polling from the egui frame loop.
    #[inline]
    pub fn snapshot(&self) -> PatchProgressSnapshot {
        PatchProgressSnapshot {
            phase: self.phase.load(Ordering::Relaxed),
            index: self.index.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }

    /// Converts the current progress state into localized UI text.
    ///
    /// Mirrors the old `status_to_text` behavior while avoiding the need to
    /// continuously allocate and synchronize full `Status` values.
    ///
    /// `error` is passed separately because terminal errors are intentionally
    /// stored outside the atomic hot path.
    pub fn text(&self, i18n: &crate::i18n::I18nMap, start_time: std::time::Instant) -> String {
        let snapshot = self.snapshot();
        let steps = phase::STEPS;

        let progress_key = match snapshot.phase {
            phase::GENERATING_FNIS_PATCHES => Some(I18nKey::StatusGeneratingFnisPatches),
            phase::READING_PATCHES => Some(I18nKey::StatusReadingPatches),
            phase::PARSING_PATCHES => Some(I18nKey::StatusParsingPatches),
            phase::APPLYING_PATCHES => Some(I18nKey::StatusApplyingPatches),
            phase::GENERATING_HKX_FILES => Some(I18nKey::StatusGeneratingHkxFiles),
            _ => None,
        };
        if let Some(key) = progress_key {
            return format!("[{}/{steps}] {} ({}/{})", snapshot.phase, i18n.t(key), snapshot.index, snapshot.total,);
        }

        match snapshot.phase {
            phase::DONE => {
                let elapsed = start_time.elapsed().as_secs_f32();
                format!("[{steps}/{steps}] {} ({elapsed:.2}s)", i18n.t(I18nKey::StatusDone))
            }

            phase::ERROR => {
                let elapsed = start_time.elapsed().as_secs_f32();
                format!(
                    "[Error] {} ({elapsed:.2}s) {}",
                    i18n.t(I18nKey::StatusError),
                    self.error.read().as_deref().unwrap_or("Unknown error"),
                )
            }

            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PatchProgressSnapshot {
    pub phase: u8,
    pub index: usize,
    pub total: usize,
}

/// The mod code of the user patch: what the conflict tool writes, and what loads last.
pub const USER_PATCH_CODE: &str = "upatch";

/// What the conflict check names the user patch by. It is not in the mod list, which can be reordered
/// and is saved: a switch turns it on, and it always loads last. See `apply_user_patch` of the settings.
pub const USER_PATCH_ID: &str = "user_patch";

/// Where the user patch is written, in the patch page's output directory: each profile has its own
/// output, and its own conflicts. Not among the mods, like MO2's overwrite.
///
/// e.g. `D:/out` -> `D:/out/.d_merge/user_patch`
#[inline]
pub fn user_patch_output(output_dir: &str) -> std::path::PathBuf {
    stdx::path::joined(std::path::Path::new(output_dir), &[".d_merge", "user_patch"])
}

/// The user patch's patch directory. e.g. `D:/out/.d_merge/user_patch/Nemesis_Engine/mod/upatch`
#[inline]
pub fn user_patch_mod_dir(output_dir: &str) -> std::path::PathBuf {
    stdx::path::joined(
        std::path::Path::new(output_dir),
        &[".d_merge", "user_patch", "Nemesis_Engine", "mod", USER_PATCH_CODE],
    )
}

/// A conflict's value to keep, from the patch page to the Patch Editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveRequest {
    /// What is patched, e.g. `meshes/actors/character/behaviors/0_master.bin`
    pub target: String,
    /// The change, e.g. `#0106/hkbModifierList/enable`
    pub path: String,
    /// The directory of the mod whose value is kept, e.g. `D:/mods/a/Nemesis_Engine/mod/a`
    pub mod_dir: String,
    /// `false`: the value goes in at once (`Use this`). `true`: to the editor as well, to change it.
    pub edit: bool,
    /// When editing: every mod with a value there, to make the result from. e.g., of an array, where the
    /// elements of several mods are wanted. With `keep_all_events`, the mods whose events are kept.
    pub sides: Vec<ResolveSide>,
    /// Keep the events of every mod of `sides`, instead of one mod's value. Only for a conflict that
    /// [`is_event_array_conflict`].
    pub keep_all_events: bool,
}

/// Is the conflict at `path` one of `enterNotifyEvents` / `exitNotifyEvents`?
///
/// Each mod points it to an array of events of its own (`hkbStateMachineEventPropertyArray`), and all
/// the events can be kept, instead of one mod's.
///
/// e.g. `#1821/hkbStateMachineStateInfo/enterNotifyEvents`: yes. `#0106/hkbModifierList/enable`: no.
#[must_use]
pub fn is_event_array_conflict(path: &str) -> bool {
    path.rsplit('/').next().is_some_and(|field| matches!(field, "enterNotifyEvents" | "exitNotifyEvents"))
}

/// A mod with a value at a conflict. See [`ResolveRequest::sides`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveSide {
    /// What it is shown as, e.g. `Paraglider`
    pub name: String,
    /// e.g. `D:/mods/a/Nemesis_Engine/mod/a`
    pub mod_dir: String,
}

#[cfg(test)]
mod event_array_tests {
    use super::*;

    #[test]
    fn tells_an_event_array_conflict() {
        assert!(is_event_array_conflict("#1821/hkbStateMachineStateInfo/enterNotifyEvents"));
        assert!(is_event_array_conflict("#1821/hkbStateMachineStateInfo/exitNotifyEvents"));
        assert!(!is_event_array_conflict("#0106/hkbModifierList/enable"));
    }
}
