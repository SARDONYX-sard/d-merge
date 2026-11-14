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
mod preview_panel;
mod toolbar;

use std::rc::Rc;

use gui_shared::i18n::{I18nKey, I18nMap};
use keybinding::commands;

use super::preview::PreviewEvent;
use crate::patch::{EditorResult, FocusAt, PatchTool, TabEvent, workspace::PANDORA_DOC};

impl PatchTool {
    /// Draws the editor of the selected object, or what takes its place.
    pub(in crate::patch) fn object_editor(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let Some(id) = self.sel.selected.clone() else {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisSelectObject)));
            return;
        };
        if self.views_ui(ui, i18n) {
            return; // a diff or a blame, as VS Code opens them in its editor
        }

        self.open_in_editor(&id);
        let editor_id = egui_code_editor::editor_id(("nemesis_object", self.is_user_patch, &id));
        self.pane.apply_focus_request(ui.ctx(), editor_id);
        self.focus_keys(ui, editor_id);
        if self.leaving_keys(ui, &id) {
            return;
        }

        self.pane.expire_preview(ui.ctx());
        self.refresh_preview(&id);

        let preview_event = self.side_panel(ui, i18n, &id);
        let result = self.editor_panel(ui, i18n, &id);
        self.after_editor(ui, &id, editor_id, result, preview_event);
    }

    /// Who has the keyboard: the side bar follows the editor (as VS Code's), `Space e` goes to the
    /// tree, `Ctrl+Shift+L` to the Problems panel.
    fn focus_keys(&mut self, ui: &egui::Ui, editor_id: egui::Id) {
        let has_focus = ui.memory(|m| m.has_focus(editor_id));
        self.note_editor_focus(ui.ctx(), has_focus);

        if self.keys.take(commands::EXPLORER_FOCUS) {
            self.enter_tree(ui, editor_id);
        }
        if self.keys.take(commands::PROBLEMS_TOGGLE) {
            self.toggle_problems_focus(ui, editor_id);
        } else if has_focus {
            self.problems.cursor.focused = false; // e.g. a click in the editor
        }
    }

    /// The keys that leave this object: close its tab, go back / forward, the next / previous tab.
    /// Returns whether it was left this frame.
    fn leaving_keys(&mut self, ui: &egui::Ui, id: &Rc<str>) -> bool {
        // `Ctrl+W` with the Normal keymap, as in VS Code. (Vim uses that key itself.)
        if self.keys.take(commands::TABS_CLOSE) {
            let close = TabEvent::Object(egui_widgets::TabAction::Close, id.to_string());
            self.apply_tab_event(Some(close), id);
            return true;
        }

        let back_or_forward = if self.keys.take(commands::HISTORY_BACK) {
            Some(true)
        } else if self.keys.take(commands::HISTORY_FORWARD) {
            Some(false)
        } else {
            egui_code_editor::NavHistory::<String>::mouse(ui)
        };
        if let Some(older) = back_or_forward {
            self.walk_history(older, id);
            ui.ctx().request_repaint();
            return true;
        }

        // `Ctrl+PageDown` / `Ctrl+PageUp` (`Alt+L` / `Alt+H`), as in VS Code, in every keymap.
        let next_or_previous = if self.keys.take(commands::TABS_NEXT) {
            Some(true)
        } else if self.keys.take(commands::TABS_PREVIOUS) {
            Some(false)
        } else {
            None
        };
        if let Some(forward) = next_or_previous {
            self.step_tab(id, forward);
            ui.ctx().request_repaint();
            return true;
        }
        false
    }

    /// Right of the editor: the mods' values while a conflict is merged, else the patch preview
    /// (none for a new object: it is written as is).
    fn side_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str) -> Option<PreviewEvent> {
        let is_merging = self.pane.merge_values.as_ref().is_some_and(|values| *values.id == *id);
        if is_merging {
            self.merge_panel(ui, i18n, id);
            return None;
        }

        let behavior = &self.work.workspace.behaviors[self.sel.active];
        let is_new = behavior.added.contains_key(id) || behavior.is_ref(id) || id == PANDORA_DOC;
        if is_new || !self.pane.show_preview {
            return None;
        }
        self.pane.preview_panel(ui, i18n, self.is_user_patch)
    }

    /// Does what the editor and the side panel asked for, once both are drawn.
    fn after_editor(
        &mut self,
        ui: &egui::Ui,
        id: &Rc<str>,
        editor_id: egui::Id,
        result: EditorResult,
        preview_event: Option<PreviewEvent>,
    ) {
        if let Some(view) = &mut self.pane.preview_view {
            if self.pane.follow_preview {
                view.follow_editor_line(self.pane.cursor_line, &self.pane.text);
            } else {
                view.stop_following();
            }
        }

        if self.apply_editor_result(result, id) {
            self.enter_tree(ui, editor_id);
        }

        if let Some((document, line)) = egui_code_editor::ReferenceList::window(ui.ctx(), &mut self.pane.references) {
            if *document != **id {
                self.pane.history.visit(&**id);
                self.sel.selected = Some(document.into());
            }
            self.pane.focus = Some(FocusAt::Line(line));
        }

        match preview_event {
            Some(PreviewEvent::Revert(block)) => self.revert_block(id, block),
            Some(PreviewEvent::JumpToEditor(row)) => {
                let line = self.pane.preview_view.as_ref().and_then(|view| view.editor_line_of(row, &self.pane.text));
                self.pane.focus = line.map(FocusAt::Line);
            }
            None => {}
        }
    }
}
