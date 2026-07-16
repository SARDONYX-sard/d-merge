//! Root of the application module.

mod background_image;
mod fetch;
mod log;
mod notify;
mod patch;
mod ui;

use std::sync::{Arc, atomic::AtomicBool};

use d_merge_gui_shared::{
    fetch::FetchState,
    i18n::I18nMap,
    patch::PatchProgress,
    settings::{Settings, ui::WindowGeometry},
};
use eframe::egui;
use parking_lot::RwLock;

use crate::{app::log::LogQueueLock, ui::theme::ThemeManager};

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

    // ── Custom theme panel  ──────────────────────────────────────────────────────
    /// Custom theme editor window state.
    pub theme_manager: ThemeManager,

    /// Whether the theme editor window is open.
    pub show_theme_editor: bool,

    pub bg_img_handle: Option<egui::TextureHandle>,

    // ── Notification bar ──────────────────────────────────────────────────────
    /// `(message, color)` shown in the mod-list status line.
    pub mod_list_msg: (String, egui::Color32),

    /// `(message, color)` shown in the notification bar at the bottom.
    pub notify: (String, egui::Color32),

    // ── Log viewer ────────────────────────────────────────────────────────────
    pub current_log_dir: Option<std::path::PathBuf>,

    /// Accumulated log lines tailed from the log file.
    pub log_lines: LogQueueLock,

    /// `true` once the log-tail watcher thread has been started.
    pub log_watcher_started: bool,

    /// temporary log window settings. save on exit.
    pub log_window_info: Arc<RwLock<WindowGeometry>>,

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
    pub(crate) fn new(settings: Settings, theme_manager: ThemeManager) -> Self {
        let log_window_info = settings.log.window.clone();
        let i18n =
            I18nMap::load_with_fallback(settings.ui.i18n_path.as_str()).unwrap_or_else(|e| {
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

            theme_manager,
            show_theme_editor: false,

            bg_img_handle: None,

            mod_list_msg: (String::new(), egui::Color32::WHITE),
            notify: (String::new(), egui::Color32::WHITE),

            current_log_dir: None,
            log_lines: LogQueueLock::default(),
            log_window_info: Arc::new(RwLock::new(log_window_info)),
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Background task polling ───────────────────────────────────────────
        self.paint_background(ctx);
        self.poll_fetch_result(ctx);
        self.poll_patch_result();
        self.update_window_info(ctx);
        self.handle_shortcuts(ctx);

        // ── One-shot setup (first frame only) ─────────────────────────────────
        self.start_log_watcher();

        // ── Top panels (outermost -> innermost) ────────────────────────────────
        self.ui_top_options(ctx);
        self.ui_paths(ctx);

        // ── Bottom panels (innermost -> outermost) ─────────────────────────────
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.log.window = self.log_window_info.read().clone();
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
        update_window_geometry(ctx, egui::ViewportId::ROOT, &mut self.settings.ui.window);
    }
}

pub(crate) fn update_window_geometry(
    ctx: &egui::Context,
    viewport_id: egui::ViewportId,
    geometry: &mut WindowGeometry,
) {
    // NOTE: Writing directly to `geometry` inside this closure would
    // deadlock (egui holds an internal lock during `input()`).
    let (pos, size, maximized) = ctx.input(|i| {
        let mut temp_pos = None;
        let mut temp_size = None;
        let mut temp_maximized = None;

        if let Some(info) = i.raw.viewports.get(&viewport_id) {
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

    if !geometry.maximized {
        if let Some(pos) = pos {
            geometry.pos_x = pos.x;
            geometry.pos_y = pos.y;
        }

        if let Some(size) = size {
            geometry.width = size.x;
            geometry.height = size.y;
        }
    }

    if let Some(maximized) = maximized {
        geometry.maximized = maximized;
    }
}
