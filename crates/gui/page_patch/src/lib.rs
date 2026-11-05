//! Patch page: choose the mods and their order, then merge them into behavior files.
//!
//! ```text
//! Mode: (VFS) (Manual)  Runtime: [SE ▼]  Parser: [Strict ▼]  [ ] Debug output ...    <- options
//! Skyrim  [D:/.../Data          ] [Auto] [Select]                                    <- paths
//! Output  [./d_merge_output     ]        [Select]
//! Mods [search...] [Clear] [All ▼] [Normalize] [Lock] ⟳ fetched (0.12 s)             <- mod list
//! ☑ | id | name | type | site | priority
//! [Log level ▼] [Log dir] [Log] [Clear] [Patch]                                      <- bottom
//! ```
//!
//! # Layout of this crate
//! - logic, testable without egui: [`fetch`] (read the mod list in the background), [`run`] (run
//!   the merge), [`visible_mods`] (filter, sort, range selection)
//! - drawing: [`ui`]

mod fetch;
mod run;
mod ui;
mod visible_mods;

use std::sync::atomic::AtomicBool;

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{DataMode, LogSettings, pages::patch::PatchSettings},
};

use self::{
    fetch::{FetchEvent, ModListFetch},
    run::PatchRun,
    visible_mods::VisibleMods,
};

/// What the page borrows from the app to draw one frame.
#[derive(Debug)]
pub struct PatchPageCtx<'a> {
    /// Frame (background) of the page's panels, from the app theme.
    pub panel_frame: egui::Frame,
    /// Persisted page state (`settings.pages.patch`).
    pub settings: &'a mut PatchSettings,
    /// The log level box and the log directory button.
    pub log: &'a mut LogSettings,
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// Runs the merge.
    pub runtime: &'a tokio::runtime::Runtime,
    /// The app's log window: toggled by the log button.
    pub show_log_window: &'a AtomicBool,
}

/// What [`PatchPage::poll`] needs. It runs on every page: a merge goes on while another is shown.
#[derive(Debug)]
pub struct PatchPollCtx<'a> {
    pub settings: &'a mut PatchSettings,
    pub i18n: &'a I18nMap,
    pub notify: &'a mut Notification,
    pub runtime: &'a tokio::runtime::Runtime,
}

/// Page state that is not persisted.
#[derive(Debug)]
pub struct PatchPage {
    fetch: ModListFetch,
    run: PatchRun,
    visible: VisibleMods,
    /// Mirrors the enabled state of all visible mods: the "check all" box in the table header.
    check_all: bool,
    /// The id of the mod a Shift+click selects from.
    selection_anchor: Option<String>,
    /// Last table width: a change resets the column widths for one frame.
    prev_table_width: f32,
    /// The status next to the reload button, e.g. `Fetched (0.12 s)`.
    list_message: Notification,
    /// The page was not drawn yet: reads the mod list and finds the Skyrim directory.
    first_frame: bool,
}

impl Default for PatchPage {
    fn default() -> Self {
        Self {
            fetch: ModListFetch::default(),
            run: PatchRun::default(),
            visible: VisibleMods::default(),
            check_all: false,
            selection_anchor: None,
            prev_table_width: 0.0,
            list_message: Notification::default(),
            first_frame: true,
        }
    }
}

impl PatchPage {
    /// Takes in the results of the background work. Call once per frame, on every page.
    pub fn poll(&mut self, egui_ctx: &egui::Context, ctx: PatchPollCtx<'_>) {
        if let Some(event) = self.fetch.poll() {
            self.on_fetch_event(event, egui_ctx, ctx.settings, ctx.i18n, ctx.notify, ctx.runtime);
        }
        self.run.poll(ctx.notify, ctx.i18n);
    }

    /// Reads the mod list of the current mode in the background.
    fn fetch_mods(&self, settings: &PatchSettings) {
        self.fetch.start(settings.current_skyrim_data_dir().to_owned(), settings.behavior.mode == DataMode::Vfs);
    }

    fn on_fetch_event(
        &mut self,
        event: FetchEvent,
        egui_ctx: &egui::Context,
        settings: &mut PatchSettings,
        i18n: &I18nMap,
        notify: &mut Notification,
        runtime: &tokio::runtime::Runtime,
    ) {
        let timed = |key, elapsed: std::time::Duration| format!("{} ({:.2} s)", i18n.t(key), elapsed.as_secs_f32());
        match event {
            FetchEvent::Fetching => self.list_message.info(i18n.t(I18nKey::ModsListFetchStateFetching)),
            FetchEvent::Fetched { mods, elapsed } => {
                let new_mods =
                    gui_shared::settings::pages::patch::mod_item::inherit_reorder_cast(settings.mod_list(), mods);
                self.check_all = new_mods.iter().all(|m| m.enabled);
                *settings.mod_list_mut() = new_mods;
                self.visible.invalidate();
                self.list_message.success(timed(I18nKey::ModsListFetchStateDone, elapsed));

                if settings.behavior.auto_run {
                    settings.mod_list_mut().iter_mut().for_each(|m| m.enabled = true);
                    self.run.start(egui_ctx, settings, notify, i18n, runtime);
                }
            }
            FetchEvent::Empty { elapsed } => self.list_message.info(timed(I18nKey::ModsListFetchStateEmpty, elapsed)),
            FetchEvent::Failed { elapsed } => self.list_message.error(timed(I18nKey::ModsListFetchStateError, elapsed)),
        }
    }

    /// Finds the Skyrim data directory in the Windows registry. A new one reloads the mod list.
    ///
    /// The registry only exists on Windows: elsewhere this reports "platform not supported".
    fn detect_vfs_data_dir(&self, settings: &mut PatchSettings, notify: &mut Notification, i18n: &I18nMap) {
        match skyrim_data_dir::get_skyrim_data_dir(settings.behavior.target_runtime) {
            Ok(dir) => {
                let new_path = dir.display().to_string();
                if settings.vfs.skyrim_data_dir != new_path {
                    settings.vfs.skyrim_data_dir = new_path;
                    self.fetch_mods(settings);
                }
            }
            Err(err) => {
                tracing::error!(%err);
                #[cfg(target_os = "windows")]
                let message = i18n.t(I18nKey::NotifyErrWindowsRegistryNotFound);
                #[cfg(not(target_os = "windows"))]
                let message = i18n.t(I18nKey::NotifyErrPlatformNotSupported);
                notify.error(message.to_owned());
            }
        }
    }
}
