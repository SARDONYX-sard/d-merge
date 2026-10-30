//! Key binding styles ("keymaps").
//!
//! A keymap may take keyboard input **before** the `TextEdit` sees it (e.g., Vim's Normal mode)
//! and edit the text itself. [`Keymap::Normal`] takes nothing: plain egui editing.
//!
//! # Adding a keymap (e.g., Emacs)
//! 1. add a variant to [`Keymap`]
//! 2. add a module (like [`vim`]) with a state type and a `handle(ui, id, text) -> KeymapOutput`
//! 3. add its state to [`KeymapState`] and dispatch it in each `match` below
//!
//! Everything else in the editor only talks to [`KeymapState`], so no other file changes.

pub(crate) mod vim;

pub use self::vim::VimOptions;
use crate::EditorCommand;

/// Key binding style of the editor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Keymap {
    /// Plain editing (like most text fields / VS Code without extensions).
    #[default]
    Normal,
    /// Vim emulation (see [`vim`] for the supported subset).
    Vim,
}

/// What a keymap did this frame.
#[derive(Debug, Default)]
pub(crate) struct KeymapOutput {
    /// The text was modified.
    pub(crate) changed: bool,
    /// A request for the host (e.g., Vim `:w` -> save).
    pub(crate) command: Option<EditorCommand>,
    /// Open the hover popup at the text cursor (e.g., Vim `K`).
    pub(crate) show_hover: bool,
}

/// Per-document state of every keymap (kept even when switching, so e.g. Vim registers survive).
#[derive(Debug, Default)]
pub struct KeymapState {
    vim: vim::VimState,
}

impl KeymapState {
    /// Vim settings, e.g., the Insert-mode escape sequences (`jk`, ...).
    pub const fn vim_options_mut(&mut self) -> &mut vim::VimOptions {
        &mut self.vim.options
    }

    /// Runs the keymap. Call **before** showing the `TextEdit` with id `id`.
    pub(crate) fn handle(
        &mut self,
        keymap: Keymap,
        ui: &egui::Ui,
        id: egui::Id,
        text: &mut String,
    ) -> KeymapOutput {
        match keymap {
            Keymap::Normal => KeymapOutput::default(),
            Keymap::Vim => self.vim.handle(ui, id, text),
        }
    }

    /// Is the user typing text right now? Completion / signature help only make sense then.
    ///
    /// e.g., `false` in Vim's Normal mode, where letters are commands.
    pub(crate) fn is_inserting(&self, keymap: Keymap) -> bool {
        match keymap {
            Keymap::Normal => true,
            Keymap::Vim => self.vim.mode() == vim::Mode::Insert,
        }
    }

    /// Char index to draw a block cursor at, if the keymap wants one.
    pub(crate) fn block_cursor(
        &self,
        keymap: Keymap,
        current: Option<usize>,
        text: &str,
    ) -> Option<usize> {
        match keymap {
            Keymap::Normal => None,
            Keymap::Vim => self.vim.block_cursor(current, text),
        }
    }

    /// Text for a status line (e.g., `-- INSERT --`), or `None` if the keymap has no modes.
    pub fn status(&self, keymap: Keymap) -> Option<String> {
        match keymap {
            Keymap::Normal => None,
            Keymap::Vim => Some(self.vim.status()),
        }
    }
}
