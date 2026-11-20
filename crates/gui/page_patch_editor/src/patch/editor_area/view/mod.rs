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
//! This is one editor group: there may be several, side by side, one above the other, or in windows
//! of their own, each with its tabs (`dock.rs`, [`super::groups`]). The preview is a tab of one of
//! them.
//!
//! A diff or a blame of the history takes the editor's place: [`super::revision_view`].

mod bottom;
mod dock;
mod editor;
mod graph;
mod merge;
pub(super) mod preview;
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
    tool::{file::FileAction, keys::Keys, selection::Selection, work_state::Work},
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
    /// An object other than the open one was edited (in the node graph): its problems are found
    /// again.
    pub(in crate::patch) edited: Option<String>,
}

impl EditorArea {
    /// Draws the editor groups (see [`super::groups`]): each the editor of its selected object, or
    /// what takes its place.
    pub(in crate::patch) fn show(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        problems: (&mut Problems, &[crate::patch::workspace::history::ObjectChange]),
    ) -> EditorOut {
        self.groups_ui(ui, i18n, (work, sel, keys), problems)
    }

    /// One group, the one being drawn ([`EditorArea`]'s `drawing`): the editor of its selected
    /// object, a diff / blame, or the side tab.
    pub(super) fn group_ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        (problems, changes): (&mut Problems, &[crate::patch::workspace::history::ObjectChange]),
    ) -> EditorOut {
        let mut out = EditorOut::default();
        let group = self.drawing;
        let active = group == self.groups.active;
        let side = self.groups.has_side(group).then(|| (self.side_title(i18n), self.groups.side_shown(group)));
        self.strip = super::groups::TabStrip { side, ..Default::default() };

        let Some(id) = sel.selected.clone() else {
            if self.groups.has_side(group) {
                self.side_only_ui(ui, i18n, work, sel);
            } else {
                // An empty group, as VS Code's: closed by its `✖` while there are others.
                let closable = self.groups.dock.groups().len() > 1;
                egui::CentralPanel::default().show(ui, |ui| {
                    if closable && close_group_button(ui, i18n) {
                        self.groups.to_close = Some(group);
                    }
                    ui.weak(i18n.t(I18nKey::ObjectSelectHint));
                });
                if active {
                    self.back_to_closed(ui, (work, sel, keys));
                }
            }
            return out;
        };
        if !self.groups.side_shown(group)
            && let Some(view) = self.views_ui(ui, i18n, (work, sel, keys))
        {
            out.file = view.file;
            return out; // a diff or a blame, as VS Code opens them in its editor
        }

        // Another object opened here (e.g. from the tree): it is shown, not the side tab.
        let opened = self.pane.opened.as_ref().is_some_and(|(behavior, open)| *behavior == sel.active && **open == *id);
        if !opened && self.pane.opened.is_some() && self.groups.side_shown(group) {
            self.groups.side = Some((group, false));
        }
        self.open_in_editor(work, sel.active, &id);
        let editor_id = egui_code_editor::editor_id(("nemesis_object", work.user_patch, &id));
        self.pane.apply_focus_request(ui.ctx(), editor_id);
        self.focus_keys(ui, (keys, problems), editor_id, &mut out);
        if self.leaving_keys(ui, (work, sel, keys), &id) {
            return out;
        }

        if active {
            self.pane.expire_preview(ui.ctx());
            self.refresh_preview((work, problems), sel.active, &id);
        }

        let result = self.editor_panel(ui, i18n, (work, sel, keys), (problems, changes), &id);
        let preview_event = if active { self.side_event.take() } else { None };
        self.after_editor((work, sel, keys), (&id, editor_id), (result, preview_event), &mut out);
        out
    }

    /// A group with no object, the side tab alone: its bar, and the side tab's content.
    fn side_only_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, work: &mut Work, sel: &Selection) {
        egui::CentralPanel::default().show(ui, |ui| {
            let names = work.names(sel.active);
            let marks = super::tab_bar::TabMarks { focused: false, problems: None, changes: ("", &[]) };
            let mut hide = false;
            let event = ui
                .horizontal(|ui| {
                    // `▥` as in the editor's toolbar: here it turns the preview off.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button = egui_widgets::shadcn::Button::selectable(true, "")
                            .icon(egui_widgets::shadcn::Icon::Columns2)
                            .small();
                        hide = ui.add(button).on_hover_text(i18n.t(I18nKey::PreviewHide)).clicked();
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            super::tab_bar::tab_bar(
                                ui,
                                i18n,
                                &self.tabs,
                                (sel.active, ""),
                                &names.graph,
                                (marks, &mut self.strip),
                                &[],
                            )
                        })
                        .inner
                    })
                    .inner
                })
                .inner;
            if hide {
                self.side_action(egui_widgets::TabAction::Close);
            }
            if let Some(TabEvent::Side(action)) = event {
                self.side_action(action);
            }
            self.side_ui(ui, i18n, work, sel);
        });
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

    /// Nothing shown (every tab closed): `Alt+←` or the mouse's back button opens the last one again.
    /// (`Ctrl+O` is Vim's in an editor, which is not there.)
    fn back_to_closed(&mut self, ui: &egui::Ui, (work, sel, keys): (&Work, &mut Selection, &mut Keys)) {
        let back = keys.take(commands::HISTORY_BACK) || egui_code_editor::NavHistory::<String>::mouse(ui) == Some(true);
        if !back {
            return;
        }
        let behavior = &work.workspace.behaviors[sel.active];
        while let Some(target) = self.pane.history.walk_from_nowhere(true) {
            if behavior.text(&target).is_some() {
                sel.selected = Some(target.into());
                self.pane.focus = Some(FocusAt::Keep);
                self.moved = true;
                return;
            }
        }
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

    /// Does what the editor and the side panel asked for, once both are drawn.
    fn after_editor(
        &mut self,
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
        out.edited = done.edited;

        // `F4` / `]q`: the next of the references, as a quickfix list.
        let step = reference_step(keys);
        let stepped = step.and_then(|forward| self.pane.references.as_mut()?.step(forward));
        if let Some((document, line)) = stepped.map(|(document, line)| (document.to_owned(), line)) {
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

/// `............ ✖` at the top right of an empty group: returns whether it was clicked.
fn close_group_button(ui: &mut egui::Ui, i18n: &I18nMap) -> bool {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        let button = egui_widgets::shadcn::Button::new("").icon(egui_widgets::shadcn::Icon::X).small();
        ui.add(button).on_hover_text(i18n.t(I18nKey::EditorGroupCloseHover)).clicked()
    })
    .inner
}

/// `F4` / `]q` (`true`) or `Shift+F4` / `[q` (`false`): the next or the previous reference.
fn reference_step(keys: &mut Keys) -> Option<bool> {
    use keybinding::commands::{EDITOR_NEXT_REFERENCE, EDITOR_PREVIOUS_REFERENCE};
    if keys.take(EDITOR_NEXT_REFERENCE) {
        Some(true)
    } else if keys.take(EDITOR_PREVIOUS_REFERENCE) {
        Some(false)
    } else {
        None
    }
}
