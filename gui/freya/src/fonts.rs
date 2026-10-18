//! Font discovery and application.
//!
//! - [`FontMode::Default`]: freya's default font with the system fallback fonts.
//! - [`FontMode::System`]: a system font family, resolved by name.
//! - [`FontMode::File`]: a font file, registered at runtime with [`Platform::load_font`].

use std::sync::OnceLock;

use d_merge_gui_shared::settings::ui::{FontMode, FontSettings};
use font_kit::source::SystemSource;
use freya::prelude::Platform;
use rayon::prelude::*;

/// Cached list of available system font families.
static FONT_FAMILIES: OnceLock<Vec<String>> = OnceLock::new();

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
pub(crate) fn font_families() -> &'static [String] {
    FONT_FAMILIES.get_or_init(|| {
        let mut families = SystemSource::new().all_families().unwrap_or_default();

        families.par_sort_unstable();
        families.dedup();

        families
    })
}

/// Resolves the font family to use on the root element.
///
/// Returns `Ok(None)` for the default font. A font file is registered to the platform before returning its family
/// name, so this must be called after the app has been launched.
///
/// # Errors
/// Returns an error if the configured font cannot be used. The caller should fall back to the default font.
pub(crate) fn resolve_font_family(font: &FontSettings) -> Result<Option<String>, FontError> {
    match font.mode {
        FontMode::Default => Ok(None),

        FontMode::System => {
            let name = font.name.trim();
            if name.is_empty() {
                return Err(FontError::Warn("Font family is empty. Use default font.".into()));
            }
            if !font_families().iter().any(|family| family == name) {
                return Err(FontError::Warn(format!("Font family not found: {name}")));
            }
            Ok(Some(name.to_string()))
        }

        FontMode::File => {
            let path = font.path.trim();
            if path.is_empty() {
                return Err(FontError::Warn("Font file path is empty. Use default font.".into()));
            }

            let bytes = std::fs::read(path)
                .map_err(|err| FontError::Error(format!("Failed to read font({path}): {err}")))?;

            // NOTE: Use a name per path, so that switching files does not mix the faces of different fonts.
            let family = format!("d_merge_user_font:{path}");
            Platform::get().load_font(family.clone(), bytes);
            Ok(Some(family))
        }
    }
}
