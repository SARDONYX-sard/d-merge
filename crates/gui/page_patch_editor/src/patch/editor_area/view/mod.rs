//! The right side: the open object's editor, one part per file.
//!
//! ```text
//! ┌ toolbar.rs ──────────────────────────────────────┐┌ merge.rs / preview_panel.rs ─┐
//! │ ← →  [ Root ✖ ][ GetUpModList ● ]   ⇆ ▲▼ 2/5  ▥  ││ the mods' values (a conflict)│
//! ├ editor.rs ───────────────────────────────────────┤│ or the patch preview         │
//! │ <hkobject name="#0106" ...>                      ││                              │
//! ├ bottom.rs ───────────────────────────────────────┤│                              │
//! │ Problems ✖ 1                                     ││                              │
//! │ NORMAL  Ln 2, Col 1        ✖ 1 ⚠ 0   Vim ▾  ⚙    ││                              │
//! └──────────────────────────────────────────────────┘└──────────────────────────────┘
//! ```
//! A diff or a blame of the history takes the editor's place: [`super::revision_view`].

mod bottom;
mod editor;
mod merge;
mod preview;
mod preview_panel;
mod problems;
mod toolbar;

use std::rc::Rc;

use gui_shared::i18n::{I18nKey, I18nMap};
use keybinding::commands;

use self::preview::PreviewEvent;
use crate::patch::{
    EditorResult, FocusAt, TabEvent,
    editor_area::EditorArea,
    problems::Problems,
    tool::{keys::Keys, list::FileAction, selection::Selection, work_state::Work},
    workspace::PANDORA_DOC,
};

/// What the editor asks of the rest: the side bar, the file of an object.
#[derive(Debug, Default)]
pub(in crate::patch) struct EditorOut {
    /// The editor got the keyboard: the side bar shows its object again.
    pub(in crate::patch) reveal: bool,
    /// The keyboard goes to the object tree, out of this editor (`Space e`).
    pub(in crate::patch) enter_tree: Option<egui::Id>,
    /// The Problems panel took the keyboard: the tree has it no more.
    pub(in crate::patch) leave_tree: bool,
    /// Copy the path of / reveal the file of an object.
    pub(in crate::patch) file: Option<(FileAction, String)>,
}

impl EditorArea {
    /// Draws the editor of the selected object, or what takes its place.
    pub(in crate::patch) fn show(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        problems: &mut Problems,
    ) -> EditorOut {
        let mut out = EditorOut::default();
        let Some(id) = sel.selected.clone() else {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisSelectObject)));
            return out;
        };
        if let Some(view) = self.views_ui(ui, i18n, (work, sel, keys)) {
            out.file = view.file;
            return out; // a diff or a blame, as VS Code opens them in its editor
        }

        self.open_in_editor(work, sel.active, &id);
        let editor_id = egui_code_editor::editor_id(("nemesis_object", work.user_patch, &id));
        self.pane.apply_focus_request(ui.ctx(), editor_id);
        self.focus_keys(ui, (keys, problems), editor_id, &mut out);
        if self.leaving_keys(ui, (work, sel, keys), &id) {
            return out;
        }

        self.pane.expire_preview(ui.ctx());
        self.refresh_preview((work, problems), sel.active, &id);

        let preview_event = self.side_panel(ui, i18n, (work, sel.active), &id);
        let result = self.editor_panel(ui, i18n, (work, sel, keys), problems, &id);
        self.after_editor(ui, (work, sel, keys), (&id, editor_id), (result, preview_event), &mut out);
        out
    }

    /// Who has the keyboard: the side bar follows the editor (as VS Code's), `Space e` goes to the
    /// tree, `Ctrl+Shift+L` to the Problems panel.
    fn focus_keys(
        &mut self,
        ui: &egui::Ui,
        (keys, problems): (&mut Keys, &mut Problems),
        editor_id: egui::Id,
        out: &mut EditorOut,
    ) {
        let has_focus = ui.memory(|m| m.has_focus(editor_id));
        if self.pane.note_focus(has_focus) {
            out.reveal = true;
            ui.ctx().request_repaint(); // the side bar is drawn first: next frame
        }

        if keys.take(commands::EXPLORER_FOCUS) {
            out.enter_tree = Some(editor_id);
        }
        if keys.take(commands::PROBLEMS_TOGGLE) {
            out.leave_tree |= self.toggle_problems_focus(ui, problems, editor_id);
        } else if has_focus {
            problems.cursor.focused = false; // e.g. a click in the editor
        }
    }

    /// `Ctrl+Shift+L`, as VS Code's `Ctrl+Shift+M`: the Problems panel shows and gets the keyboard;
    /// if it has it, it hides and the editor gets it back. See [`egui_code_editor::problems`].
    /// Returns whether the panel took the keyboard.
    fn toggle_problems_focus(&mut self, ui: &egui::Ui, problems: &mut Problems, editor: egui::Id) -> bool {
        if problems.cursor.focused {
            problems.cursor.focused = false;
            problems.open = false;
            self.pane.focus = Some(FocusAt::Keep);
            return false;
        }
        problems.open = true;
        if problems.list.is_none() {
            return false; // Not checked yet: no panel to take the keyboard.
        }
        problems.cursor.focused = true;
        ui.memory_mut(|m| m.surrender_focus(editor));
        true
    }

    /// The keys that leave this object: close its tab, go back / forward, the next / previous tab.
    /// Returns whether it was left this frame.
    fn leaving_keys(
        &mut self,
        ui: &egui::Ui,
        (work, sel, keys): (&Work, &mut Selection, &mut Keys),
        id: &Rc<str>,
    ) -> bool {
        // `Ctrl+W` with the Normal keymap, as in VS Code. (Vim uses that key itself.)
        if keys.take(commands::TABS_CLOSE) {
            let close = TabEvent::Object(egui_widgets::TabAction::Close, id.to_string());
            self.apply_tab_event(sel, Some(close), id);
            return true;
        }

        let back_or_forward = if keys.take(commands::HISTORY_BACK) {
            Some(true)
        } else if keys.take(commands::HISTORY_FORWARD) {
            Some(false)
        } else {
            egui_code_editor::NavHistory::<String>::mouse(ui)
        };
        if let Some(older) = back_or_forward {
            self.walk_history((&work.workspace.behaviors[sel.active], sel), older, id);
            ui.ctx().request_repaint();
            return true;
        }

        // `Ctrl+PageDown` / `Ctrl+PageUp` (`Alt+L` / `Alt+H`), as in VS Code, in every keymap.
        let next_or_previous = if keys.take(commands::TABS_NEXT) {
            Some(true)
        } else if keys.take(commands::TABS_PREVIOUS) {
            Some(false)
        } else {
            None
        };
        if let Some(forward) = next_or_previous {
            self.step_tab(sel, id, forward);
            ui.ctx().request_repaint();
            return true;
        }
        false
    }

    /// Right of the editor: the mods' values while a conflict is merged, else the patch preview
    /// (none for a new object: it is written as is).
    fn side_panel(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, active): (&mut Work, usize),
        id: &str,
    ) -> Option<PreviewEvent> {
        let is_merging = self.pane.merge_values.as_ref().is_some_and(|values| *values.id == *id);
        if is_merging {
            self.pane.merge_panel(ui, i18n, (work, active), id);
            return None;
        }

        let behavior = &work.workspace.behaviors[active];
        let is_new = behavior.added.contains_key(id) || behavior.is_ref(id) || id == PANDORA_DOC;
        if is_new || !self.pane.show_preview {
            return None;
        }
        self.pane.preview_panel(ui, i18n, work.user_patch)
    }

    /// Does what the editor and the side panel asked for, once both are drawn.
    fn after_editor(
        &mut self,
        ui: &egui::Ui,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        (id, editor_id): (&Rc<str>, egui::Id),
        (result, preview_event): (EditorResult, Option<PreviewEvent>),
        out: &mut EditorOut,
    ) {
        if let Some(view) = &mut self.pane.preview_view {
            if self.pane.follow_preview {
                view.follow_editor_line(self.pane.cursor_line, &self.pane.text);
            } else {
                view.stop_following();
            }
        }

        let done = self.apply_editor_result((work, sel, keys), result, id);
        if done.focus_tree {
            out.enter_tree = Some(editor_id);
        }
        out.file = out.file.take().or(done.file);

        let reference = egui_code_editor::ReferenceList::window(ui.ctx(), &mut self.pane.references);
        if let Some((document, line)) = reference.map(|(document, line)| (document.to_string(), line)) {
            self.open_location(sel, id, (&document, line));
        }

        match preview_event {
            Some(PreviewEvent::Revert(block)) => self.revert_block((work, sel.active), id, block),
            Some(PreviewEvent::JumpToEditor(row)) => {
                let line = self.pane.preview_view.as_ref().and_then(|view| view.editor_line_of(row, &self.pane.text));
                self.pane.focus = line.map(FocusAt::Line);
            }
            None => {}
        }
    }
}
