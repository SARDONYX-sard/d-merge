//! UI appearance and window-geometry settings.
//!
//! Split into two structs so that transient geometry (`window`) can be
//! updated every frame without touching the appearance fields, and vice
//! versa.
pub mod theme;

use crate::settings::{ModListUiSettings, ui::theme::CustomTheme};

/// Appearance and window settings persisted across sessions.
///
/// # JSON key
/// Serialized under the `"ui"` key in `settings.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Color theme preference.
    pub theme: theme::Theme,

    /// User-defined theme customizations.
    /// Name of the currently active preset (matches the stem of a JSON file
    /// inside the `themes/` directory next to the settings file).
    pub custom_theme: CustomTheme,

    /// background image settings
    pub background: BackgroundSettings,

    /// font settings
    pub font: FontSettings,

    /// Path to the i18n JSON file used for UI translation.
    pub i18n_path: String,

    /// Mod-list filter and sort state.
    pub mod_list: ModListUiSettings,

    /// Last-known window geometry, updated every frame and restored on startup.
    pub window: WindowGeometry,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: theme::Theme::System,
            custom_theme: CustomTheme::default(),
            background: BackgroundSettings::default(),
            font: FontSettings::default(),
            i18n_path: crate::i18n::I18nMap::FILE.into(),
            mod_list: ModListUiSettings::default(),
            window: WindowGeometry::default(),
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WindowGeometry {
    /// X coordinate of the window's top-left corner (outer rect).
    pub pos_x: f32,

    /// Y coordinate of the window's top-left corner (outer rect).
    pub pos_y: f32,

    /// Inner width of the window (excludes OS decorations).
    pub width: f32,

    /// Inner height of the window (excludes title bar and OS decorations).
    pub height: f32,

    /// Whether the window was maximized when the application last closed.
    pub maximized: bool,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self { pos_x: 100.0, pos_y: 100.0, width: 1280.0, height: 720.0, maximized: false }
    }
}

#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FontSettings {
    /// Determines how the application resolves the UI font.
    pub mode: FontMode,

    /// System font family name used when
    /// [`FontMode::System`] is selected.
    pub name: String,

    /// Font file path used when
    /// [`FontMode::File`] is selected.
    pub path: String,
}

/// Determines how the application resolves the UI font.
///
/// When [`Self::System`] is selected, [`UiSettings::font_name`] is used.
///
/// When [`Self::File`] is selected, [`UiSettings::font_path`] is used.
///
/// If the selected source cannot be loaded, the application falls back
/// to the built-in default font configuration.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontMode {
    /// Use the application's default font configuration.
    #[default]
    Default,

    /// Load a font from a system font family.
    System,

    /// Load a font from a font file path.
    ///
    /// Path to a custom `.ttc`, `.ttf` or `.otf` font file.
    File,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BackgroundSettings {
    /// Whether the background image is shown.
    pub enabled: bool,

    /// background image path
    pub path: String,
}
