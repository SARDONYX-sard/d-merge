//! Custom theme presets of the freya GUI.
//!
//! A preset is saved as `<themes_dir>/freya/<name>.json`:
//! ```json
//! { "dark": true, "colors": { "primary": "#4D6277", "background": "#0C1018D5", ... } }
//! ```
//! The format differs from the egui presets(`<themes_dir>/*.json`), so they live in a sub directory.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use d_merge_gui_shared::settings::ui::theme::Rgba;
use freya::prelude::*;

/// Name of the theme built from a preset or edited by the theme editor.
pub(crate) const CUSTOM_THEME_NAME: &str = "d-merge-custom";

/// One editable color of [`ColorsSheet`].
pub(crate) struct ColorField {
    pub name: &'static str,
    pub get: fn(&ColorsSheet) -> Color,
    pub set: fn(&mut ColorsSheet, Color),
}

macro_rules! color_fields {
    ($($field:ident),* $(,)?) => {
        /// Every color of [`ColorsSheet`].
        pub(crate) const COLOR_FIELDS: &[ColorField] = &[
            $(ColorField {
                name: stringify!($field),
                get: |colors| colors.$field,
                set: |colors, color| colors.$field = color,
            }),*
        ];
    };
}

color_fields![
    primary,
    secondary,
    tertiary,
    success,
    warning,
    error,
    info,
    background,
    surface_primary,
    surface_secondary,
    surface_tertiary,
    surface_inverse,
    surface_inverse_secondary,
    surface_inverse_tertiary,
    border,
    border_focus,
    border_disabled,
    text_primary,
    text_secondary,
    text_placeholder,
    text_inverse,
    text_highlight,
    focus,
    active,
    disabled,
    overlay,
    shadow,
];

/// Serialized form of a custom theme.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ThemePreset {
    /// Base of the component styles: dark or light.
    pub dark: bool,
    /// `field name -> color`. Missing fields keep the base theme color.
    pub colors: BTreeMap<String, Rgba>,
}

impl ThemePreset {
    pub(crate) fn from_theme(theme: &Theme, dark: bool) -> Self {
        let colors = COLOR_FIELDS
            .iter()
            .map(|field| {
                let color = (field.get)(&theme.colors);
                (field.name.to_string(), Rgba::new(color.r(), color.g(), color.b(), color.a()))
            })
            .collect();
        Self { dark, colors }
    }

    pub(crate) fn to_theme(&self) -> Theme {
        let mut theme = if self.dark { dark_theme() } else { light_theme() };
        theme.name = CUSTOM_THEME_NAME;

        for field in COLOR_FIELDS {
            if let Some(rgba) = self.colors.get(field.name) {
                (field.set)(&mut theme.colors, Color::from_argb(rgba.a, rgba.r, rgba.g, rgba.b));
            }
        }
        theme
    }
}

/// `<themes_dir>/freya`
pub(crate) fn presets_dir(themes_dir: &str) -> PathBuf {
    Path::new(themes_dir).join("freya")
}

/// Stems of the saved presets, sorted.
pub(crate) fn list_presets(themes_dir: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(presets_dir(themes_dir)) else {
        return vec![];
    };

    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("json")))
        .filter_map(|path| Some(path.file_stem()?.to_str()?.to_string()))
        .collect();
    names.sort_unstable();
    names
}

/// # Errors
/// Returns an error message if the preset cannot be read or parsed.
pub(crate) fn load_preset(themes_dir: &str, name: &str) -> Result<ThemePreset, String> {
    let path = presets_dir(themes_dir).join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    sonic_rs::from_str(&text).map_err(|err| format!("Failed to parse {}: {err}", path.display()))
}

/// # Errors
/// Returns an error message if the preset cannot be written.
pub(crate) fn save_preset(
    themes_dir: &str,
    name: &str,
    preset: &ThemePreset,
) -> Result<PathBuf, String> {
    let dir = presets_dir(themes_dir);
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Failed to create {}: {err}", dir.display()))?;

    let path = dir.join(format!("{name}.json"));
    let text = sonic_rs::to_string_pretty(preset)
        .map_err(|err| format!("Failed to serialize theme: {err}"))?;
    std::fs::write(&path, text)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    Ok(path)
}
