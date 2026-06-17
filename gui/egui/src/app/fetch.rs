//! Background mod-list fetcher.

use std::sync::Arc;

use d_merge_gui_shared::{fetch::FetchState, i18n::I18nKey, mod_item, settings::DataMode};
use egui::Color32;
use rayon::prelude::*;

use crate::app::App;

impl App {
    /// Triggers an asynchronous refresh of the mod list.
    ///
    /// Immediately sets [`FetchState::Fetching`] and updates the status bar,
    /// then spawns a worker thread.  Any in-flight fetch is implicitly
    /// superseded — the worker always writes to the same shared state, so only
    /// the last result matters.
    pub(crate) fn update_mod_list(&self) {
        tracing::debug!("`update_mod_list` has been called.");

        *self.fetch_state.write() = FetchState::Fetching;

        let dir = self.settings.current_skyrim_data_dir().to_owned();
        let use_vfs = self.settings.behavior.mode == DataMode::Vfs;
        let state = Arc::clone(&self.fetch_state);
        let fetched_mod_info = Arc::clone(&self.fetched_mod_info);

        let start_time = std::time::Instant::now();

        std::thread::spawn(move || {
            // NOTE: If rayon saturates all CPU threads the UI freezes.
            // `mod_info::get_all` must not spawn unbounded rayon work internally.
            let new_state = match mod_info::get_all(&dir, use_vfs) {
                Ok(mod_info) if mod_info.is_empty() => {
                    FetchState::Empty { elapsed: start_time.elapsed() }
                }
                Ok(mod_info) => {
                    *fetched_mod_info.write() = mod_info;
                    FetchState::Done { elapsed: start_time.elapsed() }
                }
                Err(e) => {
                    tracing::error!(%e, "mod_info::get_all error");
                    FetchState::Error { elapsed: start_time.elapsed() }
                }
            };
            *state.write() = new_state;
        });
    }

    /// Polls shared fetch state and applies any completed result to `App`.
    ///
    /// Must be called once per frame from [`eframe::App::update`].
    /// Uses `try_read` so it never blocks the UI thread.
    pub(crate) fn poll_fetch_result(&mut self, ctx: &egui::Context) {
        let Some(state) = self.fetch_state.try_read() else {
            return;
        };

        match *state {
            FetchState::Fetching => {
                self.mod_list_msg = (
                    self.i18n.t(I18nKey::ModsListFetchStateFetching).to_string(),
                    crate::app::patch::EGUI_RIGHT_BLUE,
                );
            }
            FetchState::Done { elapsed } => {
                let elapsed_secs = elapsed.as_secs_f32();
                drop(state);

                let mod_info = core::mem::take(&mut *self.fetched_mod_info.write());
                let new_mods = mod_item::inherit_reorder_cast(self.settings.mod_list(), mod_info);
                self.check_all = new_mods.par_iter().all(|m| m.enabled);
                *self.settings.mod_list_mut() = new_mods;

                // Reset to Idle so this branch is not re-entered next frame.
                *self.fetch_state.write() = FetchState::Idle;
                self.last_fetch_was_empty = false;

                self.mod_list_msg = (
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        self.i18n.t(I18nKey::ModsListFetchStateDone)
                    ),
                    Color32::GREEN,
                );

                if self.settings.behavior.auto_run {
                    self.settings.mod_list_mut().par_iter_mut().for_each(|m| m.enabled = true);
                    self.patch(ctx);
                }
            }

            FetchState::Empty { elapsed } => {
                let elapsed_secs = elapsed.as_secs_f32();
                drop(state);

                *self.fetch_state.write() = FetchState::Idle;
                self.last_fetch_was_empty = true;

                self.mod_list_msg = (
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        self.i18n.t(I18nKey::ModsListFetchStateEmpty)
                    ),
                    Color32::WHITE,
                );
            }

            FetchState::Error { elapsed } => {
                let elapsed_secs = elapsed.as_secs_f32();
                drop(state);

                *self.fetch_state.write() = FetchState::Idle;

                self.mod_list_msg = (
                    format!(
                        "{} ({elapsed_secs:.2} s)",
                        self.i18n.t(I18nKey::ModsListFetchStateError)
                    ),
                    Color32::RED,
                );
            }

            FetchState::Idle => {}
        }
    }

    /// Detects the Skyrim Data directory from the Windows registry and updates
    /// [`AppSettings::vfs_skyrim_data_dir`].
    ///
    /// No-ops if the detected path is identical to the stored one (avoids a
    /// redundant mod-list refresh).
    ///
    /// # Platform
    /// The registry read only works on Windows.  On other platforms the error
    /// branch fires immediately and shows a "platform not supported" message.
    pub(crate) fn update_vfs_skyrim_data_dir_by_reg(&mut self) {
        match skyrim_data_dir::get_skyrim_data_dir(self.settings.behavior.target_runtime) {
            Ok(dir) => {
                let new_path = dir.display().to_string();
                if self.settings.vfs.skyrim_data_dir != new_path {
                    self.settings.vfs.skyrim_data_dir = new_path;
                    self.update_mod_list();
                }
            }
            Err(err) => {
                tracing::error!(%err);
                #[cfg(target_os = "windows")]
                let err_msg = self.i18n.t(I18nKey::NotifyErrWindowsRegistryNotFound).to_string();
                #[cfg(not(target_os = "windows"))]
                let err_msg = self.i18n.t(I18nKey::NotifyErrPlatformNotSupported).to_string();
                self.notify_error(err_msg);
            }
        }
    }
}
