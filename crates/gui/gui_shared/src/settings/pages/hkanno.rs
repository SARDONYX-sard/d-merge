//! hkanno (animation annotation) editor settings.
//!
//! # Backward compatibility
//! Added in v2.9. Every field has a default, so older `settings.json` files without
//! the `"hkanno"` key still load (see `#[serde(default)]` on [`super::Settings::hkanno`]).
//!
//! # What is (not) persisted
//! Only file paths and view state are saved. The annotation text itself is re-read from
//! the hkx/xml on startup, so **unsaved edits are lost on exit** (keeps `settings.json` small).

use super::convert::OutputFormat;

/// Persisted state of the hkanno editor page.
///
/// # JSON key
/// Serialized under the `"hkanno"` key in `settings.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HkannoSettings {
    /// Open tabs, in display order.
    pub tabs: Vec<HkannoTabSettings>,

    /// Index into `tabs` of the active tab.
    pub active_tab: usize,

    /// Show the XML preview pane next to the editor.
    pub show_preview: bool,
}

impl Default for HkannoSettings {
    #[inline]
    fn default() -> Self {
        Self { tabs: Vec::new(), active_tab: 0, show_preview: true }
    }
}

/// One open file.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HkannoTabSettings {
    /// The `.hkx`/`.xml` the annotations were read from.
    pub input_path: String,

    /// Where `Save` writes. e.g., `foo.modified.hkx`
    pub output_path: String,

    /// Output format used by `Save`.
    pub format: OutputFormat,

    /// Cursor position (char index into the text) to restore.
    pub cursor: usize,
}
