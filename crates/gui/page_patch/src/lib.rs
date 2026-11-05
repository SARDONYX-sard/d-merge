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
//!   the merge), [`visible_mods`] (filter, sort, range selection), [`conflicts`] (`+` and `-`)
//! - drawing: [`ui`]

mod conflicts;
mod fetch;
mod hidden;
mod run;
mod ui;
mod visible_mods;

use std::sync::atomic::AtomicBool;

use egui_widgets::notify::Notification;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    patch::{ResolveRequest, USER_PATCH_ID, user_patch_mod_dir},
    settings::{
        DataMode, LogSettings,
        pages::patch::{PatchSettings, mod_item, support_pandora::Pandora},
    },
};

use self::{
    conflicts::{ConflictScan, ScannedMod},
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

/// How often the app looks whether a check that is running has ended.
const SCAN_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);

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
    conflicts: ConflictScan,
    /// A mod was written: the check is to be done again, once nothing is running.
    /// See [`Self::follow_written_mod`].
    rescan_wanted: bool,
    /// The running check is such a one: nobody asked for it, so it says nothing.
    is_background_scan: bool,
    /// A conflict's value to keep, for the Patch Editor. See [`Self::take_resolve`].
    resolve: Option<ResolveRequest>,
    /// See [`Self::take_user_patch_toggle`].
    user_patch_toggle: bool,
    /// The window of `ui::pandora` is open.
    pandora_open: bool,
    /// Imported, and to be applied again once the VFS mod list is read: see [`Pandora::import_to`].
    pandora_import: Option<Pandora>,
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
            conflicts: ConflictScan::with_hidden(behavior_merge::HiddenPatches::default()),
            rescan_wanted: false,
            is_background_scan: false,
            resolve: None,
            user_patch_toggle: false,
            pandora_open: false,
            pandora_import: None,
        }
    }
}

impl PatchPage {
    /// Takes in the results of the background work. Call once per frame, on every page.
    pub fn poll(&mut self, egui_ctx: &egui::Context, ctx: PatchPollCtx<'_>) {
        if let Some(event) = self.fetch.poll() {
            self.on_fetch_event(event, egui_ctx, ctx.settings, ctx.i18n, ctx.notify, ctx.runtime);
        }
        if self.run.poll(ctx.notify, ctx.i18n) {
            self.conflicts.on_merge_ended();
        }
        self.conflicts.follow_output(ctx.settings.current_output_dir());
        if let Some((count, elapsed)) = self.conflicts.poll()
            && !std::mem::take(&mut self.is_background_scan)
        {
            let seconds = elapsed.as_secs_f32();
            ctx.notify.success(format!("{}: {count} ({seconds:.2}s)", ctx.i18n.t(I18nKey::ConflictDone)));
        }
        if self.conflicts.is_scanning() {
            // The worker that reads the files has no way to wake the app.
            egui_ctx.request_repaint_after(SCAN_POLL_INTERVAL);
        }
        if self.rescan_wanted && self.conflicts.can_follow() && !self.run.is_running() && !self.conflicts.is_scanning()
        {
            self.is_background_scan = true;
            self.start_scan(egui_ctx, ctx.settings, ctx.runtime, true);
        }
    }

    /// A mod was written: the check follows in the background, as it was asked for with these mods.
    ///
    /// The mod list needs no check at all: a new load order or a mod turned off only changes the
    /// flags. Only a mod turned on that the last check did not see waits for the button, as that is
    /// the user's to decide: the check has no business running while mods are being picked.
    pub const fn follow_written_mod(&mut self) {
        self.rescan_wanted = true;
    }

    /// The user asked for the window of the user patch: the app gives that to the Patch Editor.
    pub fn take_user_patch_toggle(&mut self) -> bool {
        std::mem::take(&mut self.user_patch_toggle)
    }

    /// What the user wants to keep from a conflict: the app gives it to the Patch Editor.
    pub const fn take_resolve(&mut self) -> Option<ResolveRequest> {
        self.resolve.take()
    }

    /// The user asked for the check.
    fn scan_conflicts(
        &mut self,
        egui_ctx: &egui::Context,
        settings: &PatchSettings,
        runtime: &tokio::runtime::Runtime,
    ) {
        self.start_scan(egui_ctx, settings, runtime, false);
    }

    /// Lets the merge write what each mod changes, then reads that. See [`conflicts`].
    /// `quiet`: nothing is said of its progress.
    fn start_scan(
        &mut self,
        egui_ctx: &egui::Context,
        settings: &PatchSettings,
        runtime: &tokio::runtime::Runtime,
        quiet: bool,
    ) {
        if self.run.is_running() || self.conflicts.is_scanning() {
            return;
        }
        self.rescan_wanted = false;
        let is_vfs = settings.behavior.mode == DataMode::Vfs;
        let data_dir = settings.current_skyrim_data_dir();
        let mods = settings.mod_list().iter().filter(|item| item.enabled).map(|item| ScannedMod {
            id: item.id.clone(),
            name: item.name.clone(),
            priority: item.priority,
            dir: run::dir_of_mod(data_dir, is_vfs, item),
        });
        let mut mods: Vec<ScannedMod> = mods.collect();
        if run::uses_user_patch(settings) {
            mods.push(ScannedMod {
                id: USER_PATCH_ID.to_owned(),
                name: "User Patch".to_owned(),
                priority: run::user_patch_priority(settings.mod_list()),
                dir: Some(user_patch_mod_dir(settings.current_output_dir()).display().to_string()),
            });
        }
        self.conflicts.wait_for(conflicts::debug_patches_dir(settings.current_output_dir()), mods);
        self.run.start_patch_json(egui_ctx, settings, runtime, quiet);
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
                let new_mods = mod_item::inherit_reorder_cast(settings.mod_list(), mods);
                self.check_all = new_mods.iter().all(|m| m.enabled);
                *settings.mod_list_mut() = new_mods;
                self.apply_pandora_import(settings, notify);
                self.check_all = settings.mod_list().iter().all(|m| m.enabled);
                self.visible.invalidate();
                self.list_message.success(timed(I18nKey::ModsListFetchStateDone, elapsed));

                if settings.behavior.auto_run {
                    settings.mod_list_mut().iter_mut().for_each(|m| m.enabled = true);
                    self.run.start(egui_ctx, settings, notify, i18n, runtime);
                }
            }
            FetchEvent::Empty { elapsed } => {
                self.forget_pandora_import(settings);
                self.list_message.info(timed(I18nKey::ModsListFetchStateEmpty, elapsed));
            }
            FetchEvent::Failed { elapsed } => {
                self.forget_pandora_import(settings);
                self.list_message.error(timed(I18nKey::ModsListFetchStateError, elapsed));
            }
        }
    }

    /// The VFS mod list was just read: the mods of the import that were not in it get their state.
    /// Only once: after this the order is the user's. A list of the other mode leaves it waiting.
    fn apply_pandora_import(&mut self, settings: &mut PatchSettings, notify: &mut Notification) {
        if settings.behavior.mode != DataMode::Vfs {
            return;
        }
        if let Some(Err(err)) = self.pandora_import.take().map(|pandora| pandora.apply_to(settings)) {
            notify.error(err);
        }
    }

    /// There is no VFS list to apply it to: a later reading must not undo what the user did until then.
    fn forget_pandora_import(&mut self, settings: &PatchSettings) {
        if settings.behavior.mode == DataMode::Vfs {
            self.pandora_import = None;
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
