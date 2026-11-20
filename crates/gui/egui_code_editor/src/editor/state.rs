//! What an editor keeps between frames, per document.

use egui::text::CCursorRange;

use super::layout;
use crate::{
    Indent, Keymap, KeymapState, Language,
    keymap::View,
    popups::{
        completion::CompletionPopup, find::FindBar, hover::HoverPopup, palette, palette::Palette,
        quick_fix::QuickFixMenu, rename::RenameBox, which_key,
    },
    search,
    text_pos::LinePos,
};

/// Per-document state that is not part of the text. Keep one per open document.
#[derive(Debug, Default)]
pub struct CodeEditorState {
    pub(crate) completion: CompletionPopup,
    pub(crate) hover: HoverPopup,
    pub(crate) quick_fix: QuickFixMenu,
    /// `Ctrl+F` / `Ctrl+G` bar (Normal keymap).
    pub(crate) find: FindBar,
    /// `Ctrl+Shift+P`.
    pub(crate) palette: Palette,
    /// `F2`.
    pub(crate) rename: RenameBox,
    /// State of the key binding styles (e.g., Vim mode and registers).
    pub keymap: KeymapState,
    /// Lines visible last frame (Vim `H`, `Ctrl+D`, ...).
    pub(crate) view: View,
    /// The editor had the keyboard focus last frame.
    pub(crate) had_focus: bool,
    pub(crate) layout: layout::LayoutCache,
    /// The minimap's bars, of the last layout. See [`super::minimap`].
    pub(crate) minimap: Option<super::minimap::Bars>,
    /// The symbol the right-click menu was opened on (the pointer moves while it is open), and the
    /// document of its definition, asked once when it opened. e.g. `(12:5, "#0106")`
    pub(crate) menu_target: Option<(LinePos, String)>,
    /// The selection (char range) when the right-click menu opened: its Copy and Cut act on it. The
    /// editor has no cursor while the menu has the keyboard.
    pub(crate) menu_selection: std::ops::Range<usize>,
    /// The selection the right button was pressed on, while that button is down.
    pub(crate) held_selection: Option<CCursorRange>,
    /// The text of a read-only editor, to undo what the keymaps do to it.
    pub(crate) read_only_text: String,
    /// What one level of indentation is, for the status bar (drawn before the editor).
    pub(crate) indent: Indent,
    /// The Vim key that waits for another, and since when.
    pub(crate) which_key_waiting: which_key::Waiting,
    /// `(errors, warnings)` of the editor's own diagnostics, for the status bar.
    pub(crate) status_problems: (usize, usize),
    /// What the host shows in their place: e.g. the patch editor, the problems of a whole work. See
    /// [`Self::set_status_problems`].
    pub(crate) host_problems: Option<(usize, usize)>,
    /// The cursor and how many chars are selected, for the status bar (drawn before the editor).
    pub(crate) status_cursor: Option<(LinePos, usize)>,
    /// The menu's Paste asked the OS for the clipboard, at this time (seconds). The text arrives as
    /// an event in a later frame.
    pub(crate) pending_paste: Option<f64>,
}

impl CodeEditorState {
    /// The `(errors, warnings)` the status bar shows in place of the editor's own: what the host checks
    /// is more than the open text (e.g. a whole work, of which the open text is a part). `None`: the
    /// editor's own. Set before [`crate::status_line`] each frame.
    #[inline]
    pub const fn set_status_problems(&mut self, counts: Option<(usize, usize)>) {
        self.host_problems = counts;
    }

    /// Opens what the keys or the palette asked for: the find bar, the quick fix menu.
    pub(crate) fn open_requested_menus(
        &mut self,
        ctx: &egui::Context,
        id: egui::Id,
        text: &str,
        language: &dyn Language,
        triggers: palette::Triggers,
    ) {
        if triggers.find {
            self.find.open_find(ctx, id, text);
        } else if triggers.go_to_line {
            self.find.open_go_to_line(ctx, id);
        }
        if triggers.quick_fix
            && let Some(range) = egui::TextEdit::load_state(ctx, id).and_then(|state| state.cursor.char_range())
        {
            let pos = LinePos::from_char_index(text, range.primary.index.0);
            self.quick_fix.open(language.quick_fixes(text, pos.line, pos.column));
        }
    }

    /// Matches to highlight: the find bar (Normal) or the last `/` search (Vim).
    pub(crate) fn search_highlight(&self, keymap: Keymap) -> Option<search::Matcher> {
        match keymap {
            Keymap::Normal => self.find.highlight(),
            Keymap::Vim => self.keymap.search_highlight(keymap),
        }
    }
}
