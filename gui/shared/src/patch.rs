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

#[derive(Debug, Default)]
pub struct PatchProgress {
    /// Current phase:
    ///
    /// 0 = idle
    /// 1 = generating fnis patches
    /// 2 = reading patches
    /// 3 = parsing patches
    /// 4 = applying patches
    /// 5 = generating hkx files
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
    pub fn clear(&self) {
        self.phase.store(0, Ordering::Relaxed);
        self.index.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        *self.error.write() = None;
    }

    /// Returns a lightweight snapshot of the current progress state.
    ///
    /// Intended for UI polling from the egui frame loop.
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

        match snapshot.phase {
            1 => {
                format!(
                    "[1/6] {} ({}/{})",
                    i18n.t(I18nKey::StatusGeneratingFnisPatches),
                    snapshot.index,
                    snapshot.total,
                )
            }

            2 => {
                format!(
                    "[2/6] {} ({}/{})",
                    i18n.t(I18nKey::StatusReadingPatches),
                    snapshot.index,
                    snapshot.total,
                )
            }

            3 => {
                format!(
                    "[3/6] {} ({}/{})",
                    i18n.t(I18nKey::StatusParsingPatches),
                    snapshot.index,
                    snapshot.total,
                )
            }

            4 => {
                format!(
                    "[4/6] {} ({}/{})",
                    i18n.t(I18nKey::StatusApplyingPatches),
                    snapshot.index,
                    snapshot.total,
                )
            }

            5 => {
                format!(
                    "[5/6] {} ({}/{})",
                    i18n.t(I18nKey::StatusGeneratingHkxFiles),
                    snapshot.index,
                    snapshot.total,
                )
            }

            6 => {
                let elapsed = start_time.elapsed().as_secs_f32();
                format!("[6/6] {} ({elapsed:.2}s)", i18n.t(I18nKey::StatusDone),)
            }

            7 => {
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
