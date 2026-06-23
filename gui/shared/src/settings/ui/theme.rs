use std::{
    fs,
    path::{Path, PathBuf},
};

use snafu::ResultExt;

/// RGBA color stored in configuration.
///
/// This type is UI-framework agnostic and can be safely serialized.
/// It acts as the boundary between persisted settings and rendering
/// backends such as egui.
///
/// Each channel uses the standard 8-bit range:
///
/// - `0`   = minimum intensity / fully transparent
/// - `255` = maximum intensity / fully opaque
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgba {
    /// Red channel.
    pub r: u8,

    /// Green channel.
    pub g: u8,

    /// Blue channel.
    pub b: u8,

    /// Alpha (opacity) channel.
    ///
    /// - `0`   = fully transparent
    /// - `255` = fully opaque
    pub a: u8,
}

impl Rgba {
    /// Creates a new RGBA color.
    #[inline]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Returns an opaque RGB color.
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Returns the color as an RGBA byte array.
    #[inline]
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl serde::Serialize for Rgba {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.a == 255 {
            serializer.serialize_str(&format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b,))
        } else {
            serializer.serialize_str(&format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r, self.g, self.b, self.a,
            ))
        }
    }
}

impl<'de> serde::Deserialize<'de> for Rgba {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let s = s.strip_prefix('#').unwrap_or(&s);

        fn parse_hex<E>(s: &str) -> Result<u8, E>
        where
            E: serde::de::Error,
        {
            u8::from_str_radix(s, 16).map_err(E::custom)
        }

        match s.len() {
            6 => Ok(Self {
                r: parse_hex(&s[0..2])?,
                g: parse_hex(&s[2..4])?,
                b: parse_hex(&s[4..6])?,
                a: 255,
            }),
            8 => Ok(Self {
                r: parse_hex(&s[0..2])?,
                g: parse_hex(&s[2..4])?,
                b: parse_hex(&s[4..6])?,
                a: parse_hex(&s[6..8])?,
            }),
            _ => Err(serde::de::Error::custom("expected color in format #RRGGBB or #RRGGBBAA")),
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

    /// use Custom theme
    Custom,
}

impl Theme {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::System => "💻 System",
            Self::Dark => "🌙 Dark",
            Self::Light => "☀ Light",
            Self::Custom => "🎨 Custom",
        }
    }
}
impl core::fmt::Display for Theme {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─── Custom Theme types ──────────────────────────────────────────────────────────

/// Persisted theme settings stored inside the application settings file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CustomTheme {
    /// ./themes/
    pub themes_dir: String,

    /// Name of the currently active preset (matches the stem of a JSON file
    /// inside the `themes/` directory next to the settings file).
    pub selected_theme: Option<String>,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self { themes_dir: "./.d_merge/themes".to_string(), selected_theme: Default::default() }
    }
}

/// Serializable application theme.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThemePreset {
    /// egui visuals configuration.
    pub visuals: VisualsConfig,

    /// shadcn theme configuration.
    pub shadcn: ShadcnThemeConfig,
}

/// Serializable egui visuals configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisualsConfig {
    pub dark_mode: bool,

    pub override_text_color: Option<Rgba>,

    pub hyperlink_color: Rgba,

    pub faint_bg_color: Rgba,

    pub extreme_bg_color: Rgba,

    pub text_edit_bg_color: Option<Rgba>,

    pub code_bg_color: Rgba,

    pub warn_fg_color: Rgba,

    pub error_fg_color: Rgba,

    pub window_fill: Rgba,

    pub window_stroke: StrokeConfig,

    pub panel_fill: Rgba,

    pub window_corner_radius: u8,

    pub menu_corner_radius: u8,

    pub button_frame: bool,

    pub striped: bool,

    pub slider_trailing_fill: bool,

    pub disabled_alpha: f32,

    pub widgets: WidgetsConfig,
}

/// Serializable widget visuals collection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WidgetsConfig {
    pub noninteractive: WidgetVisualsConfig,

    pub inactive: WidgetVisualsConfig,

    pub hovered: WidgetVisualsConfig,

    pub active: WidgetVisualsConfig,

    pub open: WidgetVisualsConfig,
}

/// Serializable widget visuals.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WidgetVisualsConfig {
    pub bg_fill: Rgba,

    pub weak_bg_fill: Rgba,

    pub bg_stroke: StrokeConfig,

    pub fg_stroke: StrokeConfig,

    pub corner_radius: u8,
}

/// Serializable stroke.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StrokeConfig {
    pub width: f32,

    pub color: Rgba,
}

/// Serializable shadcn theme.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShadcnThemeConfig {
    pub background: Rgba,

    pub foreground: Rgba,

    pub card: Rgba,

    pub card_foreground: Rgba,

    pub popover: Rgba,

    pub popover_foreground: Rgba,

    pub primary: Rgba,

    pub primary_foreground: Rgba,

    pub secondary: Rgba,

    pub secondary_foreground: Rgba,

    pub muted: Rgba,

    pub muted_foreground: Rgba,

    pub accent: Rgba,

    pub accent_foreground: Rgba,

    pub destructive: Rgba,

    pub destructive_foreground: Rgba,

    pub border: Rgba,

    pub input: Rgba,

    pub ring: Rgba,

    pub radius: f32,
}

// ─── Disk I/O ─────────────────────────────────────────────────────────────────

/// # Errors
pub fn load_preset(path: &Path) -> Result<ThemePreset, Error> {
    let text = fs::read_to_string(path).with_context(|_| IoSnafu { path })?;
    sonic_rs::from_str(&text).with_context(|_| JsonSnafu { path })
}

/// # Errors
pub fn save_preset(preset: &ThemePreset, path: &Path) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|_| IoSnafu { path: parent })?;
    }
    let text = sonic_rs::to_string_pretty(preset).with_context(|_| JsonSnafu { path })?;
    fs::write(path, text).with_context(|_| IoSnafu { path })?;
    Ok(())
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
    #[snafu(display("failed json parse: {source}"))]
    Json { path: PathBuf, source: sonic_rs::Error },

    #[snafu(display("io error: {source}"))]
    Io { path: PathBuf, source: std::io::Error },
}
