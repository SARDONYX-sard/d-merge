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

/// Toggles for each editor language feature.
///
/// Defaults match the former Tauri GUI: everything on except inlay hints and signature help.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LspOptions {
    /// Key bindings of the editor.
    pub editor_mode: EditorMode,

    /// Suggestions while typing (`Ctrl+Space` to open manually).
    pub completion: bool,
    /// Quick fixes for diagnostics.
    pub code_action: bool,
    /// Error/warning underlines.
    pub diagnostics: bool,
    /// `Format` button / `Shift+Alt+F`.
    pub formatter: bool,
    /// Explanations when hovering a token.
    pub hover: bool,
    /// Parameter names shown at the end of each line.
    pub inlay_hints: bool,
    /// Parser-based coloring on top of the basic highlighting.
    pub semantic_tokens: bool,
    /// Shows the signature of the event being typed.
    pub signature_help: bool,
}

impl Default for LspOptions {
    fn default() -> Self {
        Self {
            editor_mode: EditorMode::Normal,

            code_action: true,
            completion: true,
            diagnostics: true,
            formatter: true,
            hover: true,
            inlay_hints: false,
            semantic_tokens: true,
            signature_help: false,
        }
    }
}
