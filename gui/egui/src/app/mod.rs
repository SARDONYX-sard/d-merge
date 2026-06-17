//! Root of the application module.

pub(crate) mod fetch;
pub(crate) mod log;
pub(crate) mod notify;
pub(crate) mod patch;
pub(crate) mod ui;

use std::sync::{Arc, atomic::AtomicBool};

use d_merge_gui_shared::{
    fetch::FetchState, i18n::I18nMap, patch::PatchProgress, settings::Settings,
};
use eframe::egui;
use parking_lot::RwLock;

/// Central application state.
pub(crate) struct App {
    // ── Persisted settings ────────────────────────────────────────────────────
    /// All user-configurable settings; serialized to disk on exit.
    pub settings: Settings,

    // ── i18n ─────────────────────────────────────────────────────────────────
    /// Loaded translation map.  Replaced in-place on hot-reload.
    i18n: I18nMap,

    // ── Async / threading ─────────────────────────────────────────────────────
    /// Tokio runtime for `nemesis_merge::behavior_gen`.
    pub async_rt: tokio::runtime::Runtime,

    // ── UI state (not persisted) ──────────────────────────────────────────────
    /// `true` only during the very first call to [`eframe::App::update`].
    /// Used to trigger auto-detection of the Skyrim data directory.
    pub is_first_render: bool,

    /// Controls visibility of the help / about window.
    pub show_help: bool,

    /// `true` when DnD reordering is disabled (filter active or non-priority sort).
    /// Drives the lock button in the search panel.
    pub is_locked: bool,

    /// Mirrors the enabled state of all *visible* mods.
    /// Drives the "check all" header checkbox.
    pub check_all: bool,

    /// Last observed table width; used to detect window resize and reset
    /// column widths for one frame.
    pub prev_table_available_width: f32,

    // ── Notification bar ──────────────────────────────────────────────────────
    /// `(message, color)` shown in the mod-list status line.
    pub mod_list_msg: (String, egui::Color32),

    /// `(message, color)` shown in the notification bar at the bottom.
    pub notify: (String, egui::Color32),

    // ── Log viewer ────────────────────────────────────────────────────────────
    pub current_log_dir: Option<std::path::PathBuf>,

    /// Accumulated log lines tailed from the log file.
    pub log_lines: Arc<RwLock<Vec<String>>>,

    /// `true` once the log-tail watcher thread has been started.
    pub log_watcher_started: bool,

    /// Shared with the deferred log-viewer viewport; toggled by the log button.
    pub show_log_window: Arc<AtomicBool>,

    // ── Fetch state ───────────────────────────────────────────────────────────
    /// Current phase of the background mod-list fetch.
    /// Written by the worker thread, read by the UI thread every frame.
    pub fetch_state: Arc<RwLock<FetchState>>,

    /// Staging buffer: the worker writes here before transitioning to `Done`.
    pub fetched_mod_info: Arc<RwLock<Vec<mod_info::ModInfo>>>,

    /// `true` when the last completed fetch returned zero items.
    /// Causes the table to render against an empty list while preserving
    /// the stored check-state.
    pub last_fetch_was_empty: bool,

    // ── Patch state ───────────────────────────────────────────────────────────
    /// Wall-clock instant at which the current patch run started.
    /// `None` when no patch is in progress.
    pub patch_start_time: Option<std::time::Instant>,

    /// Latest status written by the `nemesis_merge` status-report callback.
    pub patch_status: Arc<PatchProgress>,
}

impl App {
    /// Constructs `App` from persisted settings.
    ///
    /// Loads the i18n map from `i18n_path`, falling back to
    /// the built-in English strings on failure.  All background-task state
    /// starts in its idle/empty variant.
    pub(crate) fn from_settings(settings: Settings) -> Self {
        let i18n = I18nMap::load(settings.ui.i18n_path.as_str()).unwrap_or_else(|e| {
            tracing::error!("Failed to load i18n map: {e}\nFallback to default");
            I18nMap::new()
        });

        Self {
            settings,
            i18n,
            #[expect(clippy::unwrap_used)]
            async_rt: tokio::runtime::Runtime::new().unwrap(),

            is_first_render: true,
            show_help: false,
            is_locked: false,
            check_all: false,
            prev_table_available_width: 0.0,

            mod_list_msg: (String::new(), egui::Color32::WHITE),
            notify: (String::new(), egui::Color32::WHITE),

            current_log_dir: None,
            log_lines: Arc::new(RwLock::new(Vec::new())),
            log_watcher_started: false,
            show_log_window: Arc::new(AtomicBool::new(false)),

            fetch_state: Arc::new(RwLock::new(FetchState::Idle)),
            fetched_mod_info: Arc::new(RwLock::new(Vec::new())),
            last_fetch_was_empty: false,

            patch_status: Arc::new(PatchProgress::default()),
            patch_start_time: None,
        }
    }
}

impl eframe::App for App {
    /// Main frame loop.
    ///
    /// Called by eframe once per frame.  Order is strict:
    /// - Background tasks are polled first so UI reflects the latest state.
    /// - Top panels must be registered before the central panel.
    /// - Bottom panels must be registered before the central panel.
    /// - The central panel must be last.
    /// - Floating windows (viewports, modals) are registered after all panels.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Background task polling ───────────────────────────────────────────
        self.poll_fetch_result(ctx);
        self.poll_patch_result();
        self.update_window_info(ctx);

        // ── One-shot setup (first frame only) ─────────────────────────────────
        self.start_log_watcher(ctx);

        // ── Top panels (outermost → innermost) ────────────────────────────────
        self.ui_execution_mode(ctx);
        self.ui_paths(ctx);

        // ── Bottom panels (innermost → outermost) ─────────────────────────────
        // NOTE: egui stacks bottom panels in reverse registration order.
        // `ui_notification` is registered first so it appears below `ui_bottom_panel`.
        self.ui_notification(ctx);
        self.ui_bottom_panel(ctx);

        // ── Central panel (must be last among panels) ──────────────────────────
        self.ui_mod_list(ctx);

        // ── Floating windows ───────────────────────────────────────────────────
        self.ui_log_window(ctx);
        self.ui_help_window(ctx);

        self.is_first_render = false;
    }

    /// Called by eframe just before the process exits.
    ///
    /// Persists [`AppSettings`] to disk.  Tokio tasks and background threads
    /// are abandoned at this point — they hold no resources that require
    /// explicit cleanup.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

impl App {
    /// Reads the current window geometry from egui and persists it to
    /// [`AppSettings`] so the window reopens in the same position and size.
    ///
    /// Position and size are only updated when the window is **not**
    /// maximized, to avoid saving the transient geometry produced during
    /// minimize/restore cycles.
    fn update_window_info(&mut self, ctx: &egui::Context) {
        let (pos, size, maximized) = ctx.input(|i| {
            // NOTE: Writing directly to `self` inside this closure would
            // deadlock (egui holds an internal lock during `input()`).
            let mut temp_pos = None;
            let mut temp_size = None;
            let mut temp_maximized = None;

            if let Some(info) = i.raw.viewports.get(&egui::ViewportId::ROOT) {
                temp_maximized = Some(info.maximized.unwrap_or(false));

                if let Some(inner_rect) = info.inner_rect {
                    temp_size = Some(inner_rect.size());
                }
                if let Some(outer_rect) = info.outer_rect {
                    temp_pos = Some(outer_rect.min);
                }
            }

            (temp_pos, temp_size, temp_maximized)
        });

        if !self.settings.ui.window.maximized {
            if let Some(pos) = pos {
                self.settings.ui.window.pos_x = pos.x;
                self.settings.ui.window.pos_y = pos.y;
            }
            if let Some(size) = size {
                self.settings.ui.window.width = size.x;
                self.settings.ui.window.height = size.y;
            }
        }
        if let Some(max) = maximized {
            self.settings.ui.window.maximized = max;
        }
    }
}
