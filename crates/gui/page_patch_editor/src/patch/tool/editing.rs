//! The open object: its tabs, what it shows beside the text, and what the editor asked for.

use std::{path::Path, rc::Rc, time::Instant};

use egui_code_editor::{CodeEditorState, Diagnostic, EditorCommand, InlineDiff};
use egui_widgets::TabAction;
use gui_shared::settings::ui::editor::{EditorMode, LspOptions};

use super::tab_events;
use crate::patch::{EditorResult, FocusAt, PatchTool, TabEvent, anim, editor, workspace, workspace::PANDORA_DOC};

/// The editor of the open object, and what is shown beside it.
///
/// ```text
/// [tabs]                                             (PatchTool::tabs)
/// ┌ editor (text, state) ──────────┬ preview / merge ┐
/// │ underlines: problems            │ preview_view    │
/// │ inline diff: inline_diff        │ merge_values    │
/// └─────────────────────────────────┴─────────────────┘
/// [status bar: mode, lsp]
/// ```
#[derive(Debug, Default)]
pub(in crate::patch) struct EditorPane {
    /// The text in the editor.
    pub(in crate::patch) text: String,
    /// What `text` belongs to: `(behavior, id)`.
    pub(in crate::patch) opened: Option<(usize, String)>,
    pub(in crate::patch) state: CodeEditorState,
    /// The editor had the keyboard last frame. See [`PatchTool::note_editor_focus`].
    pub(in crate::patch) had_focus: bool,
    /// Where the keyboard goes next frame.
    pub(in crate::patch) focus: Option<FocusAt>,
    /// The line of the cursor, last frame.
    pub(in crate::patch) cursor_line: usize,
    /// The patch of the open object and the parser's word on it. `None`: to compute again, see
    /// [`PatchTool::refresh_preview`].
    pub(in crate::patch) preview: Option<(Option<String>, Result<(), String>)>,
    /// When the text was last typed: the preview waits for a pause.
    pub(in crate::patch) typed_at: Option<Instant>,
    /// The problems of the open object, underlined in the editor.
    pub(in crate::patch) problems: Vec<Diagnostic>,
    pub(in crate::patch) inline_diff: InlineDiff,
    pub(in crate::patch) show_inline_diff: bool,
    pub(in crate::patch) show_preview: bool,
    /// The preview scrolls with the cursor.
    pub(in crate::patch) follow_preview: bool,
    pub(in crate::patch) preview_view: Option<editor::preview::Preview>,
    /// What the merge panel shows, and the lines accepted last. See [`editor::merge`].
    pub(in crate::patch) merge_values: Option<editor::resolve::MergeValues>,
    pub(in crate::patch) merge_accepted: Vec<String>,
    /// The window of the references of an object.
    pub(in crate::patch) references: Option<egui_code_editor::ReferenceList>,
    /// Back and forward, as `Alt+←` / `Alt+→`.
    pub(in crate::patch) history: egui_code_editor::NavHistory<String>,
    pub(in crate::patch) mode: EditorMode,
    pub(in crate::patch) lsp: LspOptions,
    pub(in crate::patch) show_lsp_options: bool,
    /// `(behavior, values)`: enum / flag values per field, for the completion.
    pub(in crate::patch) field_values: Option<(usize, Rc<editor::language::FieldValues>)>,
    /// What each line of an animation data file is. See [`anim::lines`].
    pub(in crate::patch) anim_roles: Vec<anim::lines::LineRole>,
    /// The trigger names of the clip data template, read once; and those known now, for the completion.
    pub(in crate::patch) template_triggers: Option<std::collections::BTreeSet<String>>,
    pub(in crate::patch) trigger_names: Vec<String>,
}
impl PatchTool {
    /// The next / previous tab, the views' after the objects'. `current`: the object shown, if no
    /// view is. See [`tab_events::step`].
    pub(in crate::patch) fn step_tab(&mut self, current: &str, forward: bool) {
        let outcome = tab_events::step((&self.tabs, &mut self.views), self.active, current, forward);
        self.apply_tab_outcome(outcome);
    }

    /// Vim `Ctrl+O` / `Ctrl+I`: the previous / next visited object (skips deleted ones).
    pub(in crate::patch) fn walk_history(&mut self, older: bool, current: &str) {
        let mut current = current.to_string();
        while let Some(target) = self.pane.history.walk(older, &current) {
            if self.work.workspace.behaviors[self.active].text(&target).is_some() {
                self.selected = Some(target.into());
                self.pane.focus = Some(FocusAt::Keep);
                return;
            }
            current = target;
        }
    }

    /// Opens `document` at `line` (go to definition, a click in the references window).
    pub(in crate::patch) fn open_location(&mut self, current: &str, document: &str, line: usize) {
        if document != current {
            self.pane.history.visit(current);
            self.selected = Some(document.into());
        }
        self.pane.focus = Some(FocusAt::Line(line));
    }

    /// Double click or menu in the tree and the list: shows `id` in a tab that stays.
    pub(in crate::patch) fn keep_open(&mut self, id: &str) {
        let current = self.selected.as_deref().filter(|selected| *selected != id);
        self.tabs.open(self.active, id, current);
        self.tabs.pin(self.active, id);
        self.selected = Some(id.into());
    }

    /// What the editor shows, to find it again in another workspace: the behavior's `meshes_path`
    /// (e.g. `meshes/actors/character/behaviors/0_master.bin`) and the object id (e.g. `#0106`).
    pub(super) fn editor_place(&self) -> Option<(String, Rc<str>)> {
        let behavior = self.work.workspace.behaviors.get(self.active)?;
        Some((behavior.meshes_path.clone(), self.selected.clone()?))
    }

    /// Shows the place of [`Self::editor_place`] again, if the work still has it, with the keyboard in
    /// the editor at its cursor.
    pub(super) fn reopen_editor_place(&mut self, (meshes_path, id): &(String, Rc<str>)) {
        let found = self.work.workspace.behaviors.iter().position(|behavior| behavior.meshes_path == *meshes_path);
        let Some(index) = found.filter(|&index| self.work.workspace.behaviors[index].text(id).is_some()) else {
            return;
        };
        self.active = index;
        self.tabs.open(index, id, None);
        self.selected = Some(Rc::clone(id));
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// Opens the Pandora patch file of the active behavior. The first time, it is written from what
    /// the work has changed so far.
    pub(in crate::patch) fn open_pandora_doc(&mut self) {
        let code = self.work.place.mod_code.trim().to_owned();
        let behavior = &mut self.work.workspace.behaviors[self.active];
        if behavior.pandora_doc.is_none() {
            let doc = workspace::pandora::draft_doc(behavior, &code);
            behavior.pandora_doc_ids = pandora_patch::doc::doc_object_ids(&doc);
            behavior.pandora_doc = Some(doc);
            self.work.mark_dirty();
        }
        if let Some(current) = self.selected.as_deref().filter(|id| *id != PANDORA_DOC) {
            self.pane.history.visit(current);
        }
        self.selected = Some(PANDORA_DOC.into());
    }

    /// Loads `id` into the editor if it is not the open object already.
    pub(in crate::patch) fn open_in_editor(&mut self, id: &str) {
        if self.pane.opened.as_ref().is_some_and(|(active, open)| *active == self.active && open == id) {
            return;
        }
        // The tab being left keeps its undo history and Vim state.
        let left = self.pane.opened.take();
        if let Some((behavior, left)) = &left {
            self.tabs.store_state(*behavior, left, std::mem::take(&mut self.pane.state));
        }
        let current = left.as_ref().filter(|(behavior, _)| *behavior == self.active).map(|(_, id)| id.as_str());
        self.tabs.open(self.active, id, current);
        self.pane.state = self.tabs.take_state(self.active, id);
        let template_dir = Path::new(&self.work.place.template_dir);
        self.pane.refresh_trigger_names(&self.work.workspace.behaviors[self.active], template_dir);

        let text = self.work.workspace.behaviors[self.active].text(id).unwrap_or_default();
        text.clone_into(&mut self.pane.text);
        self.pane.preview = None;
        self.pane.opened = Some((self.active, id.to_owned()));
    }

    /// Recomputes the preview of `id` if it is due, and then the work's problems of it. See
    /// [`EditorPane::refresh_preview`].
    pub(in crate::patch) fn refresh_preview(&mut self, id: &str) {
        if self.pane.preview.is_some() {
            return;
        }
        self.sync_pandora_doc();
        let settings = super::pane::PreviewSettings {
            code: self.work.place.mod_code.trim(),
            parser_mode: self.work.parser_mode,
            export_format: self.work.workspace.export_format,
        };
        if self.pane.refresh_preview(&self.work.workspace.behaviors[self.active], id, settings) {
            self.refresh_problems(id);
        }
    }

    /// Does what the tab bar asked for. `current`: the object shown now. See [`tab_events`].
    pub(in crate::patch) fn apply_tab_event(&mut self, event: Option<TabEvent>, current: &str) {
        let tabs = (&mut self.tabs, &mut self.views);
        let outcome = match event {
            Some(TabEvent::Object(action, id)) => tab_events::object_action(tabs, self.active, (action, id), current),
            Some(TabEvent::View(action, at)) => tab_events::view_action(tabs, self.active, (action, at)),
            Some(TabEvent::File(action, id)) => {
                self.file_asked = Some((action, id));
                return;
            }
            None => return,
        };
        self.apply_tab_outcome(outcome);
    }

    /// What the tool does after a tab event: another selection, the editor's state gone.
    fn apply_tab_outcome(&mut self, outcome: tab_events::TabOutcome) {
        match outcome.select {
            tab_events::Select::Keep => {}
            tab_events::Select::Object(id) => self.selected = Some(id),
            tab_events::Select::Nothing => self.selected = None,
        }
        if outcome.close_editor {
            self.pane.opened = None;
        }
        if outcome.keep_focus {
            self.pane.focus = Some(FocusAt::Keep);
        }
    }

    /// Does what the editor asked for. Returns whether the keyboard goes to the object tree.
    pub(in crate::patch) fn apply_editor_result(&mut self, result: EditorResult, id: &str) -> bool {
        if result.changed {
            self.work.workspace.behaviors[self.active].set_text(id, &self.pane.text);
            self.pane.typed_at = Some(Instant::now());
            self.tabs.pin(self.active, id); // edited: it stays open
            self.object_changed(id);
            self.note_pandora_edit(id);
        }
        if let Some(line) = result.jump_to_line {
            self.pane.focus = Some(FocusAt::Line(line));
        }
        self.apply_tab_event(result.tab_event, id);
        if let Some(index) = result.inline_revert {
            self.revert_inline(id, index);
        }
        match result.command {
            // Vim `:w`: save the work now.
            Some(EditorCommand::Save) => self.work.autosave.save_soon(),
            Some(EditorCommand::JumpBack) => self.walk_history(true, id),
            Some(EditorCommand::JumpForward) => self.walk_history(false, id),
            Some(EditorCommand::NextTab) => self.step_tab(id, true),
            Some(EditorCommand::PreviousTab) => self.step_tab(id, false),
            Some(EditorCommand::CloseTab) => {
                self.apply_tab_event(Some(TabEvent::Object(TabAction::Close, id.to_owned())), id);
                self.pane.focus = Some(FocusAt::Keep);
            }
            Some(EditorCommand::TogglePreview) => self.pane.show_preview = !self.pane.show_preview,
            Some(EditorCommand::ToggleInlineDiff) => self.pane.show_inline_diff = !self.pane.show_inline_diff,
            Some(EditorCommand::NextChange) => self.pane.step_change(true),
            Some(EditorCommand::RevealFile) => {
                self.file_asked = Some((crate::patch::tool::list::FileAction::Reveal, id.to_owned()));
            }
            Some(EditorCommand::PreviousChange) => self.pane.step_change(false),
            Some(EditorCommand::ToggleExplorer | EditorCommand::GoToDefinition | EditorCommand::FindReferences)
            | None => {}
        }
        match result.configure {
            // The palette's "Open Keyboard Shortcuts (JSON)" comes the same way.
            Some(action) if self.keys.ui.run_action(action) => {}
            Some(command) => self.keys.ui.configure_command(command),
            None => {}
        }
        if let Some(older) = result.walk_history {
            self.walk_history(older, id);
        }
        if let Some((document, line)) = result.definition {
            self.open_location(id, &document, line);
        }
        result.focus_tree || result.command == Some(EditorCommand::ToggleExplorer)
    }

    /// `↶` in the inline diff: puts a changed place back to the template.
    pub(in crate::patch) fn revert_inline(&mut self, id: &str, index: usize) {
        let Some(template) = self.work.workspace.behaviors[self.active].template_object(id) else { return };
        let text = InlineDiff::revert(&template.text, &self.pane.text, index);
        self.replace_object_text(id, text);
    }

    /// `↶` in the preview: puts changed block `block` back to the template.
    pub(in crate::patch) fn revert_block(&mut self, id: &str, block: usize) {
        let Some(template) = self.work.workspace.behaviors[self.active].template_object(id) else { return };
        let Some(text) = self.pane.reverted_block(&template.text, block) else { return };
        self.replace_object_text(id, text);
    }

    /// Object `id` is `text` now, in the editor and in the work.
    fn replace_object_text(&mut self, id: &str, text: String) {
        self.pane.text = text;
        self.work.workspace.behaviors[self.active].set_text(id, &self.pane.text);
        self.pane.preview = None;
        self.object_changed(id);
        self.note_pandora_edit(id);
    }

    /// Object `id` was edited here: its part of the Pandora patch file follows at the next pause.
    fn note_pandora_edit(&mut self, id: &str) {
        if self.work.workspace.behaviors[self.active].pandora_doc.is_some()
            && !self.work.pandora_edited.iter().any(|edited| edited == id)
        {
            self.work.pandora_edited.push(id.to_owned());
        }
    }

    /// What was edited since the last time goes to the other side: an object to its part of the Pandora
    /// patch file, the file to the objects. After a pause in typing, as it reads the whole file.
    pub(super) fn sync_pandora_doc(&mut self) {
        let edited = std::mem::take(&mut self.work.pandora_edited);
        let active = self.active;
        for id in edited {
            let Some(behavior) = self.work.workspace.behaviors.get_mut(active) else { continue };
            if id == PANDORA_DOC {
                let (changed, _) = behavior.apply_pandora_doc();
                for changed in changed {
                    self.object_changed(&changed);
                }
            } else {
                behavior.update_pandora_doc(&id);
            }
        }
    }

    /// The text of object `id` changed, or it was added or deleted: `names` takes that one in.
    pub(in crate::patch) fn object_changed(&mut self, id: &str) {
        self.work.names.changed(id);
        self.work.mark_dirty();
    }

    /// The active behavior's names, with the changes since the last call taken in.
    ///
    /// Asked for every frame: an `Rc`, so the thousands of strings are not copied each time.
    pub(in crate::patch) fn names(&mut self) -> Rc<editor::names::Names> {
        if !self.work.names.is_of(self.active) {
            self.reset_names(); // another behavior: what was found of this one goes too
        }
        self.work.names.get(self.active, &self.work.workspace.behaviors[self.active])
    }

    /// The behaviors themselves changed (loaded, closed): `names` starts over.
    pub(in crate::patch) fn reset_names(&mut self) {
        self.work.names.reset();
        self.work.edits += 1; // what was found of the work before is not of this one
        self.problems.stale = true;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::patch::workspace::{ADSF, Behavior};

    /// A clip data "behavior" with the files `ids` (it needs no template file).
    fn clip_data(ids: &[&str]) -> Behavior {
        let mut adsf = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        for id in ids {
            adsf.added.insert((*id).to_owned(), String::new());
        }
        adsf
    }

    /// Reading the mod again: the object being edited is shown again, if it is still there.
    #[test]
    fn the_editor_place_comes_back() {
        let mut tool = PatchTool::default();
        tool.work.workspace.behaviors.push(clip_data(&["M~1/a.txt"]));
        tool.selected = Some("M~1/a.txt".into());
        let place = tool.editor_place().unwrap();

        tool.selected = None;
        tool.reopen_editor_place(&place);
        assert_eq!(tool.selected.as_deref(), Some("M~1/a.txt"));
        assert!(matches!(tool.pane.focus, Some(FocusAt::Keep)));

        // Gone from the mod: nothing is opened.
        tool.work.workspace.behaviors[0] = clip_data(&[]);
        tool.selected = None;
        tool.reopen_editor_place(&place);
        assert_eq!(tool.selected, None);
    }
}
