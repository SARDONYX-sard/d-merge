//! hkanno (animation annotation) editor settings.
//!
//! # Backward compatibility
//! Added in v2.9. Every field has a default, so older `settings.json` files without
//! the `"hkanno"` key still load (see `#[serde(default)]` on [`super::Settings::hkanno`]).
//!
//! # What is (not) persisted
//! Only file paths and view state are saved. The annotation text itself is re-read from
//! the hkx/xml on startup, so **unsaved edits are lost on exit** (keeps `settings.json` small).

use super::convert::OutFormat;

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

    /// Which language features are enabled.
    pub lsp: HkannoLspOptions,

    /// Key bindings of the editor.
    pub editor_mode: EditorMode,
}

/// Key binding style of text editors.
///
/// Serialized as `"normal"` / `"vim"` (more styles such as Emacs may be added).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorMode {
    /// Plain editing.
    #[default]
    Normal,
    /// Vim emulation.
    Vim,
}

impl EditorMode {
    /// All modes in UI order.
    pub const ALL: [Self; 2] = [Self::Normal, Self::Vim];

    /// Label shown in the UI.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Vim => "Vim",
        }
    }
}

impl Default for HkannoSettings {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            show_preview: true,
            lsp: HkannoLspOptions::default(),
            editor_mode: EditorMode::Normal,
        }
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
    pub format: OutFormat,

    /// Cursor position (char index into the text) to restore.
    pub cursor: usize,
}

/// Toggles for each editor language feature.
///
/// Defaults match the former Tauri GUI: everything on except inlay hints and signature help.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HkannoLspOptions {
    /// Suggestions while typing (`Ctrl+Space` to open manually).
    pub completion: bool,
    /// Quick fixes for diagnostics.
    pub code_action: bool,
    /// Error/warning underlines.
    pub diagnostics: bool,
    /// `Format` button / `Shift+Alt+F`.
    pub formatter: bool,
    /// Parser-based coloring on top of the basic highlighting.
    pub semantic_tokens: bool,
    /// Explanations when hovering a token.
    pub hover: bool,
    /// Parameter names shown at the end of each line.
    pub inlay_hints: bool,
    /// Shows the signature of the event being typed.
    pub signature_help: bool,
}

impl Default for HkannoLspOptions {
    fn default() -> Self {
        Self {
            completion: true,
            code_action: true,
            diagnostics: true,
            formatter: true,
            semantic_tokens: true,
            hover: true,
            inlay_hints: false,
            signature_help: false,
        }
    }
}
