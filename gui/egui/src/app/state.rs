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

/// Represents the lifecycle of a background mod-list fetch.
///
/// The background thread writes one of the terminal variants ([`Done`],
/// [`Empty`], [`Error`]) when it finishes; the UI thread polls every frame
/// and transitions back to [`Idle`] after consuming the result.
///
/// ```text
/// Idle ──start──▶ Fetching ──ok / non-empty──▶ Done
///                     │
///                     ├──ok / empty──────────▶ Empty
///                     └──error───────────────▶ Error
/// Done | Empty | Error ──consumed──▶ Idle
/// ```
///
/// [`Done`]: FetchState::Done
/// [`Empty`]: FetchState::Empty
/// [`Error`]: FetchState::Error
/// [`Idle`]: FetchState::Idle
#[derive(Debug)]
pub(crate) enum FetchState {
    /// No fetch is in progress; the mod list is up-to-date (or never loaded).
    Idle,

    /// A background worker thread is currently fetching mod info.
    Fetching,

    /// The fetch succeeded and returned at least one item.
    ///
    /// `elapsed` is the wall-clock duration of the worker call, displayed in
    /// the status bar as `"Done (0.42 s)"`.
    Done { elapsed: std::time::Duration },

    /// The fetch succeeded but the directory contained zero mod entries.
    ///
    /// The UI preserves the existing check-state rather than clearing the
    /// list, so the user does not lose their selections on an empty scan.
    Empty { elapsed: std::time::Duration },

    /// The fetch failed (I/O error, invalid path, etc.).
    ///
    /// The error is logged via `tracing::error!` inside the worker; the UI
    /// shows a red status message using the elapsed time.
    Error { elapsed: std::time::Duration },
}

#[derive(Debug, Default)]
pub(crate) struct PatchProgress {
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
    pub(crate) fn clear(&self) {
        self.phase.store(0, Ordering::Relaxed);
        self.index.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        *self.error.write() = None;
    }

    /// Applies a new patch-generation status emitted by
    /// `nemesis_merge::behavior_gen`.
    ///
    /// High-frequency progress updates are written into atomics so the egui
    /// render loop can poll them without lock contention.
    ///
    /// Terminal states:
    /// - `Done`
    /// - `Error`
    ///
    /// are stored separately because they occur rarely compared to normal
    /// progress updates.
    pub(crate) fn apply(&self, status: nemesis_merge::Status, ctx: &egui::Context) {
        let old_phase = self.phase.load(Ordering::Relaxed);

        let new_phase = match &status {
            nemesis_merge::Status::GeneratingFnisPatches { .. } => 1,
            nemesis_merge::Status::ReadingPatches { .. } => 2,
            nemesis_merge::Status::ParsingPatches { .. } => 3,
            nemesis_merge::Status::ApplyingPatches { .. } => 4,
            nemesis_merge::Status::GeneratingHkxFiles { .. } => 5,
            nemesis_merge::Status::Done => 6,
            nemesis_merge::Status::Error(_) => 7,
        };

        self.phase.store(new_phase, Ordering::Relaxed);

        match status {
            nemesis_merge::Status::GeneratingFnisPatches { index, total }
            | nemesis_merge::Status::ReadingPatches { index, total }
            | nemesis_merge::Status::ParsingPatches { index, total }
            | nemesis_merge::Status::ApplyingPatches { index, total }
            | nemesis_merge::Status::GeneratingHkxFiles { index, total } => {
                self.index.store(index, Ordering::Relaxed);
                self.total.store(total, Ordering::Relaxed);
            }

            nemesis_merge::Status::Done => {}

            nemesis_merge::Status::Error(err) => {
                *self.error.write() = Some(err);
            }
        }

        // NOTE: Request a UI repaint when the phase changes or reaches a terminal state.
        // - If we do not do this, notifications that the patch has been applied may be significantly delayed.
        // - Comparing the status with the `old` phase is also important; without this, the UI will freeze.
        if old_phase != new_phase || matches!(new_phase, 6 | 7) {
            ctx.request_repaint();
        }
    }

    /// Returns a lightweight snapshot of the current progress state.
    ///
    /// Intended for UI polling from the egui frame loop.
    fn snapshot(&self) -> PatchProgressSnapshot {
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
    pub(crate) fn text(
        &self,
        i18n: &crate::i18n::I18nMap,
        start_time: std::time::Instant,
    ) -> String {
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

    /// Returns the current UI color associated with the active patch phase.
    pub(crate) fn color(&self) -> egui::Color32 {
        let snapshot = self.snapshot();

        match snapshot.phase {
            1 => egui::Color32::from_rgb(120, 170, 255),
            2 => super::patch::EGUI_RIGHT_BLUE,
            3 => egui::Color32::from_rgb(140, 200, 255),
            4 => egui::Color32::from_rgb(255, 170, 120),
            5 => egui::Color32::from_rgb(200, 140, 255),
            6 => egui::Color32::GREEN,
            7 => egui::Color32::RED,
            _ => egui::Color32::WHITE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PatchProgressSnapshot {
    pub phase: u8,
    pub index: usize,
    pub total: usize,
}
