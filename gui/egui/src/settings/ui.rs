//! UI appearance and window-geometry settings.
//!
//! Split into two structs so that transient geometry (`window`) can be
//! updated every frame without touching the appearance fields, and vice
//! versa.

use crate::settings::ModListUiSettings;

/// Appearance and window settings persisted across sessions.
///
/// # JSON key
/// Serialized under the `"ui"` key in `settings.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct UiSettings {
    /// Color theme preference.
    ///
    /// Applied on startup via `egui::Context::set_theme` and whenever the
    /// user changes it in the execution-mode panel.
    pub theme: Theme,

    /// Whether the main window background is transparent.
    ///
    /// When `true`, all panels are built with [`egui::Frame::new()`] (no
    /// fill) so the OS window chrome shows through.  Requires the window to
    /// be created with `transparent: true` in [`eframe::NativeOptions`].
    pub transparent: bool,

    /// Path to a custom `.ttf` or `.otf` font file.
    ///
    /// `None` falls back to the built-in font bundled with the application.
    pub font_path: Option<String>,

    /// Path to the i18n JSON file used for UI translation.
    ///
    /// Defaults to the built-in English strings when absent or invalid.
    /// Can be hot-reloaded via the help window without restarting.
    pub i18n_path: String,

    /// Mod-list filter and sort state.
    pub mod_list: ModListUiSettings,

    /// Last-known window geometry, updated every frame and restored on
    /// startup.
    pub window: WindowGeometry,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            transparent: false,
            font_path: None,
            i18n_path: crate::i18n::I18nMap::FILE.into(),
            mod_list: ModListUiSettings::default(),
            window: WindowGeometry::default(),
        }
    }
}

/// Theme color
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// System default: follows the OS theme setting. This is the default if `theme` is `None`.
    System,

    /// Dark background.
    Dark,

    /// Light background.
    Light,
}

impl Theme {
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "💻 System",
            Self::Dark => "🌙 Dark",
            Self::Light => "☀ Light",
        }
    }
}

impl From<Theme> for egui::ThemePreference {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::System => Self::System,
            Theme::Dark => Self::Dark,
            Theme::Light => Self::Light,
        }
    }
}

/// Position and size of the main window in logical pixels.
///
/// Updated every frame from [`egui::ViewportInfo`] and written to disk on
/// exit so the window reopens in the same location.
///
/// Position and size are **only** updated when the window is not maximized,
/// to avoid saving the transient geometry produced during minimize/restore
/// cycles.
///
/// # JSON key
/// Serialized under `"ui"."window"` in `settings.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct WindowGeometry {
    /// X coordinate of the window's top-left corner (outer rect).
    pub pos_x: f32,

    /// Y coordinate of the window's top-left corner (outer rect).
    pub pos_y: f32,

    /// Inner width of the window (excludes OS decorations).
    pub width: f32,

    /// Inner height of the window (excludes title bar and OS decorations).
    pub height: f32,

    /// Whether the window was maximized when the application last closed.
    ///
    /// When `true`, the stored `pos_*` / `width` / `height` values are
    /// ignored at startup and the window is maximized immediately.
    pub maximized: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self { pos_x: 100.0, pos_y: 100.0, width: 1280.0, height: 720.0, maximized: false }
    }
}
