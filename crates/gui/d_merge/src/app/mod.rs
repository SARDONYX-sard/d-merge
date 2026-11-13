//! Root of the application module.

mod background_image;
mod log;
mod pages;
pub(crate) mod router;
mod ui;

use std::sync::{Arc, atomic::AtomicBool};

use eframe::egui;
use gui_shared::{
    i18n::I18nMap,
    settings::{Settings, ui::WindowGeometry},
};
use page_convert::ConvertPage;
use page_hkanno::HkannoPage;
use page_patch::{PatchPage, PatchPollCtx};
use page_patch_editor::PatchEditorPage;
use parking_lot::RwLock;

use crate::{
    app::log::LogQueueLock,
    ui::{notify::Notification, theme::ThemeManager},
};

/// Central application state.
pub(crate) struct App {
    // ── Persisted settings ────────────────────────────────────────────────────
    /// All user-configurable settings; serialized to disk on exit.
    pub settings: Settings,

    // ── i18n ─────────────────────────────────────────────────────────────────
    /// Loaded translation map.  Replaced in-place on hot-reload.
    i18n: I18nMap,

    // ── Async / threading ─────────────────────────────────────────────────────
    /// Tokio runtime for the merge and conversions.
    pub async_rt: tokio::runtime::Runtime,

    // ── UI state (not persisted) ──────────────────────────────────────────────
    /// Controls visibility of the help / about window.
    pub show_help: bool,

    // ── Custom theme panel  ──────────────────────────────────────────────────────
    /// Custom theme editor window state.
    pub theme_manager: ThemeManager,

    /// Whether the theme editor window is open.
    pub show_theme_editor: bool,

    pub bg_img_handle: Option<egui::TextureHandle>,

    // ── Notification bar ──────────────────────────────────────────────────────
    ///  shown in the notification bar at the bottom.
    pub notify: Notification,

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

    // ── Pages ─────────────────────────────────────────────────────────────────
    /// Patch page state that is not persisted (mod list fetch, merge progress, ...).
    pub patch: PatchPage,

    /// Convert page state that is not persisted (loaded tree, progress, ...).
    pub convert: ConvertPage,

    /// hkanno editor state (open files, editor buffers, preview, ...).
    pub hkanno: HkannoPage,

    /// CRC32 page state (inputs, loaded mod tree).
    pub patch_editor: PatchEditorPage,

    /// A page asked for another page to be shown. The router does that, after it drew this frame.
    pub pending_page: Option<gui_shared::settings::ui::Page>,

    // ── Key bindings ──────────────────────────────────────────────────────────
    /// The defaults and `keybindings.json`, read again when it changes.
    pub keys: keybinding::Keybindings,
}

/// The user's key bindings, next to the settings. See [`keybinding::file`].
const KEYBINDINGS_PATH: &str = "./.d_merge/keybindings.json";

impl App {
    /// Constructs `App` from persisted settings.
    ///
    /// Loads the i18n map from `i18n_path`, falling back to
    /// the built-in English strings on failure.  All background-task state
    /// starts in its idle/empty variant.
    pub(crate) fn new(settings: Settings, theme_manager: ThemeManager) -> Self {
        let log_window_info = settings.log.window.clone();
        let mut notify = Notification::default();
        let hkanno = HkannoPage::restore(&settings.pages.hkanno, &mut notify);
        let i18n = I18nMap::load_with_fallback(settings.ui.i18n_path.as_str()).unwrap_or_else(|e| {
            tracing::error!("Failed to load i18n map: {e}\nFallback to default");
            I18nMap::new()
        });

        Self {
            settings,
            i18n,
            #[expect(clippy::unwrap_used)]
            async_rt: tokio::runtime::Runtime::new().unwrap(),

            show_help: false,

            theme_manager,
            show_theme_editor: false,

            bg_img_handle: None,

            notify,

            current_log_dir: None,
            log_lines: LogQueueLock::default(),
            log_window_info: Arc::new(RwLock::new(log_window_info)),
            log_watcher_started: false,
            show_log_window: Arc::new(AtomicBool::new(false)),

            patch: PatchPage::default(),
            convert: ConvertPage::default(),
            hkanno,
            patch_editor: PatchEditorPage::default(),
            pending_page: None,
            keys: keybinding::Keybindings::load(std::path::Path::new(KEYBINDINGS_PATH)),
        }
    }

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

/// Per-frame hooks called by [`router::Root`] (which is the actual `eframe::App`).
impl App {
    /// Work done every frame **before** any page is drawn, regardless of the visible page.
    pub(crate) fn frame_begin(&mut self, ctx: &egui::Context) {
        // ── Background task polling ───────────────────────────────────────────
        self.paint_background(ctx);
        if self.patch_editor.take_mod_written() {
            self.patch.follow_written_mod();
        }
        let patch_ctx = PatchPollCtx {
            settings: &mut self.settings.pages.patch,
            i18n: &self.i18n,
            notify: &mut self.notify,
            runtime: &self.async_rt,
        };
        self.patch.poll(ctx, patch_ctx);
        self.convert.poll(&mut self.notify, &self.i18n);
        self.update_window_info(ctx);
        // `keybindings.json` changed: what of it can't be taken is told once.
        self.keys.reload_if_changed(ctx.input(|i| i.time));
        let warnings = self.keys.take_warnings();
        if !warnings.is_empty() {
            self.notify.warn(warnings.join(
                "
",
            ));
        }

        // ── One-shot setup (first frame only) ─────────────────────────────────
        self.start_log_watcher();
    }

    /// Work done every frame **after** the page: floating windows on top of everything.
    pub(crate) fn frame_end(&mut self, ctx: &egui::Context) {
        self.ui_log_window(ctx);
        self.ui_help_window(ctx);
    }

    /// Persists everything on shutdown.
    pub(crate) fn on_exit(&mut self) {
        self.settings.log.window = self.log_window_info.read().clone();
        self.hkanno.store(&mut self.settings.pages.hkanno);

        if let Err(e) = self.settings.save() {
            tracing::error!("Failed to save settings: {e}");
        }
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
