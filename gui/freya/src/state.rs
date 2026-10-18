//! Application state shared by the components.

use std::sync::Arc;

use d_merge_gui_shared::{i18n::I18nMap, patch::PatchProgress, settings::Settings};
use freya::prelude::*;

use crate::{log::LogQueueLock, theme::Appearance, ui::log_window::LogGeometry};

/// A colored message. `color: None` uses the theme text color.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Notice {
    pub text: String,
    pub color: Option<Color>,
}

impl Notice {
    pub(crate) fn new(text: impl Into<String>, color: impl Into<Option<Color>>) -> Self {
        Self { text: text.into(), color: color.into() }
    }
}

/// Reactive application state. (Provided as a context by the root component)
///
/// Every field is a [`State`], so this struct is `Copy` and can be moved into closures freely.
#[derive(Clone, Copy)]
pub(crate) struct AppState {
    // ── Persisted settings ────────────────────────────────────────────────────
    /// All user-configurable settings; saved to disk on exit.
    pub settings: State<Settings>,

    /// Loaded translation map. Replaced on hot-reload.
    pub i18n: State<I18nMap>,

    // ── Mod list ──────────────────────────────────────────────────────────────
    /// `true` while the mod list is being fetched.
    pub fetching: State<bool>,

    /// Incremented per fetch request, so that the result of an outdated fetch is ignored.
    pub fetch_generation: State<u64>,

    /// `true` when the last completed fetch returned zero items.
    /// The table then shows an empty list while the stored list(check states) is kept.
    pub last_fetch_was_empty: State<bool>,

    /// Status message of the mod list fetch.
    pub mod_list_msg: State<Notice>,

    /// The table keeps its own row order and sort.
    /// Incrementing this re-creates the table, so that the order changed outside the table
    /// (fetch, normalize, mode switch) is shown.
    pub table_generation: State<u64>,

    /// ID of the mod used as the anchor for Shift-click range selection.
    pub selection_anchor: State<Option<String>>,

    /// Whether the Shift key is held. (Pointer events do not carry the modifiers.)
    pub shift_held: State<bool>,

    /// Position of the table context menu.
    pub context_menu: State<Option<(f64, f64)>>,

    // ── Patch ─────────────────────────────────────────────────────────────────
    /// `true` while a patch is running.
    pub patching: State<bool>,

    // ── Windows ───────────────────────────────────────────────────────────────
    pub show_help: State<bool>,
    pub show_theme_editor: State<bool>,

    /// The log viewer window, if opened. (It is a separate OS window)
    pub log_window: State<Option<freya::winit::window::WindowId>>,

    // ── Appearance ────────────────────────────────────────────────────────────
    pub appearance: State<Appearance>,

    /// Font family applied to the root element. `None` is the default font.
    pub font_family: State<Option<String>>,

    /// Incremented to reload the background image from disk.
    pub background_generation: State<u64>,

    // ── Notification bar ──────────────────────────────────────────────────────
    pub notify: State<Notice>,
}

impl AppState {
    pub(crate) fn new(settings: Settings, i18n: I18nMap) -> Self {
        Self {
            settings: State::create(settings),
            i18n: State::create(i18n),

            fetching: State::create(false),
            fetch_generation: State::create(0),
            last_fetch_was_empty: State::create(false),
            mod_list_msg: State::create(Notice::default()),
            table_generation: State::create(0),
            selection_anchor: State::create(None),
            shift_held: State::create(false),
            context_menu: State::create(None),

            patching: State::create(false),

            show_help: State::create(false),
            show_theme_editor: State::create(false),
            log_window: State::create(None),

            appearance: State::create(Appearance::default()),
            font_family: State::create(None),
            background_generation: State::create(0),

            notify: State::create(Notice::default()),
        }
    }

    /// Translate `key` with the current i18n map, without subscribing to it.
    ///
    /// Use [`Self::t`] in render functions.
    pub(crate) fn t_peek(&self, key: d_merge_gui_shared::i18n::I18nKey) -> String {
        self.i18n.peek().t(key).to_string()
    }

    /// Translate `key` with the current i18n map.
    pub(crate) fn t(&self, key: d_merge_gui_shared::i18n::I18nKey) -> String {
        self.i18n.read().t(key).to_string()
    }
}

/// Non-reactive handles shared by the components.
#[derive(Clone)]
pub(crate) struct Shared {
    /// Latest status written by the `nemesis_merge` status-report callback.
    pub patch_status: Arc<PatchProgress>,

    /// Lines tailed from the log file.
    pub log_lines: LogQueueLock,

    /// Tokio runtime for `nemesis_merge::behavior_gen` and blocking I/O.
    pub runtime: tokio::runtime::Handle,

    /// Geometry of the log window. Saved when the main window closes.
    pub log_geometry: LogGeometry,
}

impl Shared {
    pub(crate) fn new(runtime: tokio::runtime::Handle, log_geometry: LogGeometry) -> Self {
        Self {
            patch_status: Arc::new(PatchProgress::default()),
            log_lines: LogQueueLock::default(),
            runtime,
            log_geometry,
        }
    }
}
