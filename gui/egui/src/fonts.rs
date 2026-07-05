//! Font discovery, caching and application utilities for egui.
//!
//! # Font resolution
//!
//! Fonts are resolved according to [`FontSettings::mode`].
//!
//! - [`FontMode::Default`] loads the built-in fallback font.
//! - [`FontMode::System`] loads a system font family.
//! - [`FontMode::File`] loads a font from a file path.
//!
//! # Caching
//!
//! Loaded font bytes are cached for the lifetime of the process to avoid
//! repeated filesystem access and font backend queries.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use d_merge_gui_shared::settings::ui::{FontMode, FontSettings};
use dashmap::DashMap;
use egui::{Context, FontData, FontDefinitions, FontFamily};
use font_kit::source::SystemSource;
use rayon::prelude::*;

/// Cached list of available system font families.
static FONT_FAMILIES: OnceLock<Vec<String>> = OnceLock::new();

/// Cached font bytes.
static FONT_CACHE: OnceLock<DashMap<FontKey, Arc<Vec<u8>>>> = OnceLock::new();

/// Internal cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FontKey {
    System(String),
    File(PathBuf),
}

/// Font loading error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FontError {
    /// Non-fatal configuration or lookup issue.
    Warn(String),

    /// Fatal font loading failure.
    Error(String),
}

/// Returns all available system font family names.
///
/// The result is cached for the lifetime of the process.
#[must_use]
pub(crate) fn font_families() -> &'static [String] {
    FONT_FAMILIES.get_or_init(|| {
        let mut families = SystemSource::new().all_families().unwrap_or_default();

        families.par_sort_unstable();
        families.dedup();

        families
    })
}

/// A font setup routine that runs only once before the GUI app is launched
pub(crate) fn setup_fonts(ctx: &Context, font: &FontSettings) {
    if let Err(err) = crate::fonts::set_fonts(ctx, font) {
        match err {
            FontError::Warn(msg) => tracing::warn!(msg),
            FontError::Error(msg) => tracing::error!(msg),
        }
    }
}

/// Applies the configured UI font.
///
/// The active font source is determined by [`FontSettings::mode`].
///
/// # Errors
///
/// Returns a [`FontError`] when the selected font cannot be resolved or
/// loaded.
pub(crate) fn set_fonts(ctx: &Context, font: &FontSettings) -> Result<(), FontError> {
    match font.mode {
        FontMode::Default => apply_default_font(ctx),

        FontMode::System => with_default_fallback(
            ctx,
            if font.name.is_empty() {
                Err(FontError::Warn("System font family name is empty".into()))
            } else {
                apply_system_font(ctx, &font.name)
            },
        ),

        FontMode::File => with_default_fallback(
            ctx,
            if font.path.is_empty() {
                Err(FontError::Warn("Font file path is empty".into()))
            } else {
                apply_font_file(ctx, Path::new(&font.path))
            },
        ),
    }
}

fn with_default_fallback(ctx: &Context, result: Result<(), FontError>) -> Result<(), FontError> {
    if result.is_err() {
        tracing::info!("font load failed -> fallback to default font");
        let _ = apply_default_font(ctx);
    }

    result
}

/// Applies the default application font.
///
/// # Errors
///
/// Returns a [`FontError`] if the default font cannot be loaded.
fn apply_default_font(ctx: &Context) -> Result<(), FontError> {
    #[cfg(target_os = "windows")]
    {
        const DEFAULT_FONT_PATH: &str = "c:/Windows/Fonts/msyh.ttc";
        apply_font_file(ctx, Path::new(DEFAULT_FONT_PATH))
    }
    #[cfg(not(target_os = "windows"))]
    {
        ctx.set_fonts(FontDefinitions::default());
        Ok(())
    }
}

/// Applies a system font family.
///
/// The font data is cached after the first successful load.
///
/// # Errors
///
/// Returns:
///
/// - [`FontError::Warn`] if the family does not exist.
/// - [`FontError::Warn`] if the family contains no fonts.
/// - [`FontError::Error`] if the font cannot be loaded.
/// - [`FontError::Error`] if font bytes cannot be extracted.
pub(crate) fn apply_system_font(ctx: &Context, family_name: &str) -> Result<(), FontError> {
    let cache = FONT_CACHE.get_or_init(DashMap::new);

    let key = FontKey::System(family_name.to_owned());

    let bytes = if let Some(bytes) = cache.get(&key) {
        bytes.clone()
    } else {
        let family = SystemSource::new()
            .select_family_by_name(family_name)
            .map_err(|_| FontError::Warn(format!("System font family not found: {family_name}")))?;

        let handle = family.fonts().first().ok_or_else(|| {
            FontError::Warn(format!("System font family contains no fonts: {family_name}"))
        })?;

        let font = handle.load().map_err(|err| {
            FontError::Error(format!("Failed to load system font {family_name}: {err}"))
        })?;

        let bytes = font
            .copy_font_data()
            .ok_or_else(|| FontError::Error(format!("Failed to copy font data: {family_name}")))?;

        cache.insert(key, Arc::clone(&bytes));

        bytes
    };

    ctx.set_fonts(build_font_definitions(bytes));

    Ok(())
}

/// Applies a font file.
///
/// The font data is cached after the first successful load.
///
/// # Errors
///
/// Returns a [`FontError::Error`] if the file cannot be resolved or read.
pub(crate) fn apply_font_file(ctx: &Context, path: &Path) -> Result<(), FontError> {
    let cache = FONT_CACHE.get_or_init(DashMap::new);

    let canonical = path.canonicalize().map_err(|err| {
        FontError::Error(format!("Failed to canonicalize font path {}: {err}", path.display()))
    })?;

    let key = FontKey::File(canonical.clone());

    let bytes = if let Some(bytes) = cache.get(&key) {
        Arc::clone(&bytes)
    } else {
        let data = std::fs::read(&canonical).map_err(|err| {
            FontError::Error(format!("Failed to read font file {}: {err}", canonical.display()))
        })?;

        let bytes = Arc::new(data);

        cache.insert(key, Arc::clone(&bytes));

        bytes
    };

    ctx.set_fonts(build_font_definitions(bytes));

    Ok(())
}

/// Builds an egui font definition using the provided font data.
///
/// The font is inserted with the highest priority in both proportional and
/// monospace families.
fn build_font_definitions(font_data: Arc<Vec<u8>>) -> FontDefinitions {
    let mut defs = FontDefinitions::default();

    defs.font_data.insert("user_font".into(), FontData::from_owned(font_data.to_vec()).into());
    defs.families.entry(FontFamily::Proportional).or_default().insert(0, "user_font".into());
    defs.families.entry(FontFamily::Monospace).or_default().insert(0, "user_font".into());

    defs
}
