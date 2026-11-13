//! A read-only diff or blame in the code editor, as VS Code's diff editor and GitLens' blame:
//!
//! ```text
//! ┌ diff ──────────────────────────────────┐   ┌ blame ──────────────────────────────────────┐
//! │   <hkobject name="#0106" ...>          │   │ template       <hkobject name="#0106"..>    │
//! │ - <hkparam name="enable">true<..>      │   │ #2 fix speed   <hkparam name="enable">..    │
//! │ + <hkparam name="enable">false<..>     │   │ working        <hkparam name="userData"..   │
//! │ NORMAL  Ln 2, Col 1                    │   └─────────────────────────────────────────────┘
//! └────────────────────────────────────────┘
//! ```
//! A diff is the newer text with the older lines in it ([`InlineDiff`]); it opens on its first
//! change. `▲ ▼` ([`DiffView::nav`]) and Vim's `]c` / `[c` go from change to change; the editor's
//! keys move in it as usual. A blame is the text with a note beside each line.
//!
//! ```text
//! let mut view = DiffView::diff(old, new);       // kept by the host, e.g. in its tab
//! view.focus();                                  // when its tab is shown
//! if let Some(line) = view.nav(ui, "Next change") { view.go_to(line) }  // `▲ ▼ 2/5`
//! let output = view.show(ui, ("diff", id), &Xml, keymap);
//! if output.command == Some(EditorCommand::CloseTab) { /* close its tab */ }
//! ```

use crate::{CodeEditor, CodeEditorState, EditorCommand, InlineDiff, Keymap, Language, text_pos::LinePos};

/// What the read-only editor of a view hosts: its changes, and its tab's keys (Vim's `Space c`,
/// `:q`, `gt`).
const COMMANDS: [EditorCommand; 5] = [
    EditorCommand::NextChange,
    EditorCommand::PreviousChange,
    EditorCommand::CloseTab,
    EditorCommand::NextTab,
    EditorCommand::PreviousTab,
];

/// See the module docs.
#[derive(Debug, Default)]
pub struct DiffView {
    text: String,
    /// A diff's changes; none for a blame.
    diff: InlineDiff,
    /// A blame's note of each line, e.g. `3 days ago  #2 fix the speed`; none for a diff.
    notes: Vec<String>,
    state: CodeEditorState,
    /// Where the cursor is: `▲ ▼` go from there.
    cursor_line: usize,
    /// The line to go to on the next [`Self::show`]: the first change when opened, or `▲ ▼`.
    go_to: Option<usize>,
    /// The editor takes the keyboard on the next [`Self::show`].
    focus: bool,
}

/// What [`DiffView::show`] gives back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffViewOutput {
    /// What its editor asked of the tabs: [`EditorCommand::CloseTab`], `NextTab`, `PreviousTab`.
    pub command: Option<EditorCommand>,
    /// Its editor has the keyboard (e.g. for the key bindings' `editorFocus`).
    pub has_focus: bool,
}

impl DiffView {
    /// The changes from `old` to `new`, opened on the first one.
    #[must_use]
    pub fn diff(old: &str, new: String) -> Self {
        let diff = InlineDiff::between(old, &new);
        Self { go_to: diff.first_line(), text: new, diff, ..Self::default() }
    }

    /// `text` with `notes[line]` beside each line.
    #[inline]
    #[must_use]
    pub fn blame(text: String, notes: Vec<String>) -> Self {
        Self { text, notes, ..Self::default() }
    }

    /// Its editor takes the keyboard when next shown: e.g. its tab was clicked.
    #[inline]
    pub const fn focus(&mut self) {
        self.focus = true;
    }

    /// Nothing to show beside the text: a diff of equal texts, or an empty blame.
    #[inline]
    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        self.diff.hunks.is_empty() && self.notes.is_empty()
    }

    /// The changes, e.g. for a test.
    #[inline]
    #[must_use]
    pub const fn changes(&self) -> &InlineDiff {
        &self.diff
    }

    /// `▲ ▼ 2/5`, drawn right to left: see [`InlineDiff::nav`]. Nothing for a blame. Returns the line
    /// a button asked for: give it to [`Self::go_to`].
    #[inline]
    pub fn nav(&self, ui: &mut egui::Ui, hover: &str) -> Option<usize> {
        self.diff.nav(ui, self.cursor_line, hover)
    }

    /// Its cursor goes to `line` on the next [`Self::show`].
    #[inline]
    pub const fn go_to(&mut self, line: usize) {
        self.go_to = Some(line);
    }

    /// Draws it, taking the space left, with its status line at the bottom.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        id_salt: impl egui::AsIdSalt + Copy,
        language: &dyn Language,
        keymap: Keymap,
    ) -> DiffViewOutput {
        let editor_id = crate::editor_id(id_salt);
        egui::Panel::bottom(("diff_view_status", id_salt)).show(ui, |ui| {
            crate::status_line(ui, &self.state, keymap, &self.text);
        });
        let focus = std::mem::take(&mut self.focus);
        if focus {
            ui.memory_mut(|m| m.request_focus(editor_id));
        }
        let output =
            egui::ScrollArea::both().id_salt(("diff_view_scroll", id_salt)).auto_shrink(false).show(ui, |ui| {
                CodeEditor::new(id_salt, &mut self.text)
                    .language(language)
                    .read_only(true)
                    .keymap(keymap)
                    .inline_diff(Some(&self.diff))
                    .line_annotations(&self.notes)
                    .host_commands(&COMMANDS)
                    .show(ui, &mut self.state)
            });
        let output = output.inner;
        // Asked again of the editor drawn: a focus asked for by id before it was may be lost.
        if focus {
            output.response.request_focus();
        }
        if let Some(cursor) = output.cursor {
            self.cursor_line = cursor.line;
        }
        let command = output.command;
        let step = match command {
            Some(EditorCommand::NextChange) => self.diff.step(self.cursor_line, true),
            Some(EditorCommand::PreviousChange) => self.diff.step(self.cursor_line, false),
            _ => None,
        };
        if let Some(line) = step.or_else(|| self.go_to.take()) {
            crate::set_cursor(ui.ctx(), editor_id, LinePos { line, column: 0 }.to_char_index(&self.text));
        }
        DiffViewOutput { command, has_focus: ui.memory(|m| m.has_focus(editor_id)) }
    }
}

impl InlineDiff {
    /// `▲ ▼ 2/5` of the changes, drawn right to left, as VS Code's diff editor. `hover`: the buttons'
    /// hover text. Returns the line to go to if one was clicked; nothing is drawn without changes.
    pub fn nav(&self, ui: &mut egui::Ui, cursor_line: usize, hover: &str) -> Option<usize> {
        let (at, count) = self.position(cursor_line)?;
        ui.label(format!("{}/{count}", at + 1));
        let mut line = None;
        for (forward, glyph) in [(true, "▼"), (false, "▲")] {
            if ui.small_button(glyph).on_hover_text(hover).clicked() {
                line = self.step(cursor_line, forward);
            }
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A diff opens on its first change; a blame has nothing to go to.
    #[test]
    fn a_diff_opens_on_its_first_change() {
        let view = DiffView::diff("a\nb\nd\n", "a\nc\nd\ne\n".to_owned());
        assert_eq!((view.go_to, view.changes().hunks.len()), (Some(1), 2));
        assert!(!view.is_unchanged());
        let blame = DiffView::blame("a\n".to_owned(), vec!["#1".to_owned()]);
        assert_eq!(blame.go_to, None);
        assert!(DiffView::diff("a\n", "a\n".to_owned()).is_unchanged());
    }
}
