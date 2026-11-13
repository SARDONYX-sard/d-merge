//! What an editor can be asked for, and which of its features are on.

use crate::EditorAction;

/// Which optional features are on. All default to `true` except inlay hints / signature help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorFeatures {
    pub completion: bool,
    pub hover: bool,
    pub diagnostics: bool,
    pub format: bool,
    pub inlay_hints: bool,
    pub signature_help: bool,
}

impl Default for EditorFeatures {
    #[inline]
    fn default() -> Self {
        Self {
            completion: true,
            hover: true,
            diagnostics: true,
            format: true,
            inlay_hints: false,
            signature_help: false,
        }
    }
}

/// Requests from the editor to its host (the host decides what they mean).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorCommand {
    /// e.g., Vim `:w`.
    Save,
    /// Vim `gd`: go to the definition of the symbol under the cursor.
    GoToDefinition,
    /// Vim `gD`: find the references of the symbol under the cursor.
    FindReferences,
    /// Vim `Ctrl+O` past the oldest jump of this text: back to the previous document / object.
    JumpBack,
    /// Vim `Ctrl+I` past the newest jump: forward again.
    JumpForward,
    /// Vim `<Space>e`: move the keyboard to the host's file / object tree.
    ToggleExplorer,
    /// Vim `<Space>l`, `gt`, `:tabn`: the host's next tab.
    NextTab,
    /// Vim `<Space>h`, `gT`, `:tabp`: the host's previous tab.
    PreviousTab,
    /// Vim `<Space>c`, `:tabc`, `:q`: close the host's tab.
    CloseTab,
    /// Vim `<Space>p`, `:preview`: show or hide what the host shows beside the editor.
    TogglePreview,
    /// Vim `<Space>d`, `:diff`: show or hide the changes inside the editor.
    ToggleInlineDiff,
    /// Vim `]c`: the host's next change.
    NextChange,
    /// Vim `[c`: the host's previous change.
    PreviousChange,
}

impl EditorCommand {
    pub(crate) const ALL: [Self; 13] = [
        Self::Save,
        Self::GoToDefinition,
        Self::FindReferences,
        Self::JumpBack,
        Self::JumpForward,
        Self::ToggleExplorer,
        Self::NextTab,
        Self::PreviousTab,
        Self::CloseTab,
        Self::TogglePreview,
        Self::ToggleInlineDiff,
        Self::NextChange,
        Self::PreviousChange,
    ];

    pub(crate) const fn action_id(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::GoToDefinition => "go_to_definition",
            Self::FindReferences => "find_references",
            Self::JumpBack => "jump_back",
            Self::JumpForward => "jump_forward",
            Self::ToggleExplorer => "toggle_explorer",
            Self::NextTab => "next_tab",
            Self::PreviousTab => "previous_tab",
            Self::CloseTab => "close_tab",
            Self::TogglePreview => "toggle_preview",
            Self::ToggleInlineDiff => "toggle_inline_diff",
            Self::NextChange => "next_change",
            Self::PreviousChange => "previous_change",
        }
    }

    pub(crate) fn from_action_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.action_id() == id)
    }

    /// This command in the palette (`Ctrl+Shift+P`). Picking it comes back as
    /// [`CodeEditorOutput::command`], the same as its key.
    #[inline]
    pub const fn action(self, label: &str) -> EditorAction<'_> {
        EditorAction { id: self.action_id(), label }
    }
}
