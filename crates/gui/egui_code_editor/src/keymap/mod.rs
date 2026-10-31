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
use crate::{EditorCommand, search::Matcher};

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
    /// How the view should scroll (e.g., Vim `G`: bring the cursor into view; `zz`: center it).
    pub(crate) scroll: Option<Scroll>,
}

/// What a keymap shows in its status line. See [`crate::status_line`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub mode: StatusMode,
    /// Details, e.g. `:w`, `/foo    [2/5]`, `recording @a  2d`, `Pattern not found: foo`
    pub text: String,
}

/// The mode shown as a colored badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    VisualBlock,
    /// Typing in the `:` / `/` / `?` prompt.
    Command,
}

impl StatusMode {
    /// e.g., `NORMAL`, `V-LINE`
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual => "VISUAL",
            Self::VisualLine => "V-LINE",
            Self::VisualBlock => "V-BLOCK",
            Self::Command => "COMMAND",
        }
    }

    /// Badge color (One Dark palette, like Neovim status lines: Normal blue, Insert green,
    /// Visual purple, Command yellow).
    pub const fn color(self) -> egui::Color32 {
        match self {
            Self::Normal => egui::Color32::from_rgb(97, 175, 239),
            Self::Insert => egui::Color32::from_rgb(152, 195, 121),
            Self::Visual | Self::VisualLine | Self::VisualBlock => {
                egui::Color32::from_rgb(198, 120, 221)
            }
            Self::Command => egui::Color32::from_rgb(229, 192, 123),
        }
    }
}

/// A scroll request from a keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scroll {
    /// Make the cursor visible, scrolling as little as possible.
    IntoView,
    /// Put the cursor line at the top / center / bottom (Vim `zt`, `zz`, `zb`).
    Align(egui::Align),
    /// Scroll the view by this many lines, positive = down (Vim `Ctrl+E`, `Ctrl+D`).
    Lines(i32),
}

/// The lines currently visible in the editor (for Vim `H`, `M`, `L`, `Ctrl+D`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct View {
    /// 0-based first visible line.
    pub(crate) first_line: usize,
    /// How many lines fit in the view.
    pub(crate) lines: usize,
}

impl Default for View {
    fn default() -> Self {
        Self { first_line: 0, lines: 30 }
    }
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
    ///
    /// `view`: the lines visible last frame.
    pub(crate) fn handle(
        &mut self,
        keymap: Keymap,
        ui: &egui::Ui,
        id: egui::Id,
        text: &mut String,
        view: View,
    ) -> KeymapOutput {
        match keymap {
            Keymap::Normal => KeymapOutput::default(),
            Keymap::Vim => self.vim.handle(ui, id, text, view),
        }
    }

    /// Hide egui's blinking text cursor (the keymap draws a block cursor instead).
    pub(crate) fn hides_text_cursor(&self, keymap: Keymap) -> bool {
        match keymap {
            Keymap::Normal => false,
            Keymap::Vim => self.vim.mode() != vim::Mode::Insert,
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

    /// The search whose matches should be highlighted (e.g., after Vim `/foo`).
    pub(crate) fn search_highlight(&self, keymap: Keymap) -> Option<Matcher> {
        match keymap {
            Keymap::Normal => None,
            Keymap::Vim => self.vim.search_highlight(),
        }
    }

    /// Char ranges of a block selection to draw, one per line (Vim `Ctrl+V`).
    pub(crate) fn block_selection(
        &self,
        keymap: Keymap,
        text: &str,
    ) -> Option<Vec<std::ops::Range<usize>>> {
        match keymap {
            Keymap::Normal => None,
            Keymap::Vim => self.vim.block_ranges(text),
        }
    }

    /// The status line (mode + details), or `None` if the keymap has no modes.
    /// Draw it with [`crate::status_line`].
    pub fn status(&self, keymap: Keymap) -> Option<StatusLine> {
        match keymap {
            Keymap::Normal => None,
            Keymap::Vim => Some(self.vim.status()),
        }
    }
}
