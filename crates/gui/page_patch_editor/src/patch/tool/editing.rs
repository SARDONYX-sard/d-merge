//! The open object: its tabs, what it shows beside the text, and what the editor asked for.

use std::{borrow::Cow, path::Path, rc::Rc, time::Instant};

use egui_code_editor::{CodeEditorState, Diagnostic, EditorCommand, InlineDiff};
use egui_widgets::TabAction;
use gui_shared::settings::ui::editor::{EditorMode, LspOptions};

use crate::patch::{
    EditorResult, FocusAt, PatchTool, REFRESH_DELAY, TabEvent, anim, editor, workspace, workspace::PANDORA_DOC,
};

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
    /// The next or previous tab, with the keyboard staying in the editor.
    /// The next / previous tab, the views' after the objects', around the ends. `current`: the
    /// object shown, if no view is.
    pub(in crate::patch) fn step_tab(&mut self, current: &str, forward: bool) {
        #[derive(Clone)]
        enum Tab {
            Object(String),
            View(usize),
        }
        let mut order: Vec<Tab> = self.tabs.of(self.active).map(|tab| Tab::Object(tab.id.clone())).collect();
        order.extend(self.views.tabs_of(self.active).into_iter().map(|tab| Tab::View(tab.at)));
        let here = order.iter().position(|tab| match (tab, self.views.list.active) {
            (Tab::View(at), Some(shown)) => *at == shown,
            (Tab::Object(id), None) => id == current,
            _ => false,
        });
        let (Some(here), len) = (here, order.len()) else { return };
        let next = if forward { (here + 1) % len } else { (here + len - 1) % len };
        match order[next].clone() {
            Tab::Object(id) => {
                self.views.list.active = None;
                self.selected = Some(id.into());
            }
            Tab::View(at) => {
                self.selected = self.views.list.get(at).map(|view| view.id.as_str().into());
                self.views.show(at);
                return; // its own editor takes the keyboard: see `Views::show`
            }
        }
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// Vim `Ctrl+O` / `Ctrl+I`: the previous / next visited object (skips deleted ones).
    pub(in crate::patch) fn walk_history(&mut self, older: bool, current: &str) {
        let mut current = current.to_string();
        while let Some(target) = self.pane.history.walk(older, &current) {
            if self.workspace.behaviors[self.active].text(&target).is_some() {
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
    pub(in crate::patch) fn editor_place(&self) -> Option<(String, Rc<str>)> {
        let behavior = self.workspace.behaviors.get(self.active)?;
        Some((behavior.meshes_path.clone(), self.selected.clone()?))
    }

    /// Shows the place of [`Self::editor_place`] again, if the work still has it, with the keyboard in
    /// the editor at its cursor.
    pub(in crate::patch) fn reopen_editor_place(&mut self, (meshes_path, id): &(String, Rc<str>)) {
        let found = self.workspace.behaviors.iter().position(|behavior| behavior.meshes_path == *meshes_path);
        let Some(index) = found.filter(|&index| self.workspace.behaviors[index].text(id).is_some()) else { return };
        self.active = index;
        self.tabs.open(index, id, None);
        self.selected = Some(Rc::clone(id));
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// Opens the Pandora patch file of the active behavior. The first time, it is written from what
    /// the work has changed so far.
    pub(in crate::patch) fn open_pandora_doc(&mut self) {
        let code = self.place.mod_code.trim().to_owned();
        let behavior = &mut self.workspace.behaviors[self.active];
        if behavior.pandora_doc.is_none() {
            let doc = workspace::pandora::draft_doc(behavior, &code);
            behavior.pandora_doc_ids = pandora_patch::doc::doc_object_ids(&doc);
            behavior.pandora_doc = Some(doc);
            self.mark_dirty();
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
        self.refresh_trigger_names();

        let text = self.workspace.behaviors[self.active].text(id).unwrap_or_default();
        text.clone_into(&mut self.pane.text);
        self.pane.preview = None;
        self.pane.opened = Some((self.active, id.to_owned()));
    }

    /// In a clip data file, trigger lines complete with the known names.
    pub(in crate::patch) fn refresh_trigger_names(&mut self) {
        let behavior = &self.workspace.behaviors[self.active];
        if !behavior.adsf {
            return;
        }
        let template_dir = Path::new(&self.place.template_dir);
        let template =
            self.pane.template_triggers.get_or_insert_with(|| anim::data::template_trigger_names(template_dir));
        self.pane.trigger_names = anim::data::trigger_names(template, behavior);
    }

    /// Gives the editor the keyboard and moves its cursor, if that was asked for.
    pub(in crate::patch) fn apply_focus_request(&mut self, ctx: &egui::Context, editor: egui::Id) {
        let Some(at) = self.pane.focus.take() else { return };
        let index = match at {
            FocusAt::Line(line) => {
                egui_code_editor::text_pos::LinePos { line, column: 0 }.to_char_index(&self.pane.text)
            }
            FocusAt::Keep => egui::TextEdit::load_state(ctx, editor)
                .and_then(|state| state.cursor.char_range())
                .map_or(0, |range| range.primary.index.0),
        };
        egui_code_editor::set_cursor(ctx, editor, index);
    }

    /// The pause after typing is over: `preview` goes, for [`Self::refresh_preview`] to compute it.
    /// Until then the last results stay on screen, as in VS Code.
    pub(in crate::patch) fn expire_preview(&mut self, ctx: &egui::Context) {
        let Some(typed_at) = self.pane.typed_at else { return };
        let left = REFRESH_DELAY.saturating_sub(typed_at.elapsed());
        if left.is_zero() {
            self.pane.typed_at = None;
            self.pane.preview = None;
        } else {
            ctx.request_repaint_after(left);
        }
    }

    /// Recomputes the patch, its check and the underlines after the text changed.
    pub(in crate::patch) fn refresh_preview(&mut self, id: &str) {
        if self.pane.preview.is_some() {
            return;
        }
        self.sync_pandora_doc();
        let behavior = &self.workspace.behaviors[self.active];
        let patch = behavior.patch(self.place.mod_code.trim(), id);
        let check = patch.as_deref().map_or(Ok(()), |patch| behavior.validate(id, patch, self.parser_mode));
        self.pane.problems = if id == PANDORA_DOC {
            editor::pandora::diagnostics(&self.pane.text, behavior)
        } else if behavior.is_anim_data() {
            self.pane.anim_roles = anim::lines::roles(behavior.adsf, id, &self.pane.text);
            Vec::new() // Only the parser's, below: see `anim_lines`.
        } else if behavior.is_ref(id) {
            Vec::new() // not this mod's to fix
        } else {
            let is_new = behavior.added.contains_key(id);
            editor::diagnostics::field_diagnostics(&self.pane.text, behavior, is_new, self.place.mod_code.trim())
        };
        // Keep the ▲ / ▼ position across edits.
        let current = self.pane.preview_view.as_ref().map_or(0, |view| view.current);
        let nemesis_view = patch.as_deref().map(editor::preview::Preview::new);
        // The parser reads the patch, whose lines are the Nemesis preview's: back to the editor's.
        if let Err(error) = &check {
            let view = nemesis_view.as_ref();
            let to_editor_line = |line| view.and_then(|view| view.editor_line_of(line, &self.pane.text));
            editor::diagnostics::add_parser_error(&mut self.pane.problems, &self.pane.text, error, to_editor_line);
        }
        // What the export would write. Animation data stays lines: Pandora's export doesn't write it.
        let as_pandora = self.workspace.export_format == workspace::ExportFormat::PandoraV5 && !behavior.is_anim_data();
        let (view, check) = if as_pandora {
            match editor::pandora::op_preview(behavior, id) {
                Ok(view) => (Some(view), Ok(())),
                Err(problem) => (None, Err(problem)),
            }
        } else {
            (nemesis_view, check)
        };
        self.pane.preview_view = view;
        if let Some(view) = &mut self.pane.preview_view {
            view.keep_current(current);
        }
        self.pane.preview = Some((patch.map(Cow::into_owned), check));
        self.pane.inline_diff = behavior
            .template_object(id)
            .map_or_else(InlineDiff::default, |template| InlineDiff::between(&template.text, &self.pane.text));
        self.refresh_problems(id);
    }

    /// Enum / flag values only depend on the template: computed once per behavior.
    pub(in crate::patch) fn field_values(&mut self) -> Rc<editor::language::FieldValues> {
        let active = self.active;
        let cached = self.pane.field_values.take().filter(|(cached, _)| *cached == active);
        let (_, values) = cached
            .unwrap_or_else(|| (active, Rc::new(editor::language::field_values(&self.workspace.behaviors[active]))));
        self.pane.field_values = Some((active, Rc::clone(&values)));
        values
    }

    /// Does what the tab bar asked for. `current`: the object shown now.
    pub(in crate::patch) fn apply_tab_event(&mut self, event: Option<TabEvent>, current: &str) {
        match event {
            Some(TabEvent::Object(action, id)) => self.object_tab_action(action, id, current),
            Some(TabEvent::View(action, at)) => self.view_tab_action(action, at),
            Some(TabEvent::File(action, id)) => self.file_asked = Some((action, id)),
            None => {}
        }
    }

    /// What an object's tab asked for. `current`: the object shown now.
    fn object_tab_action(&mut self, action: TabAction, id: String, current: &str) {
        let active = self.active;
        match action {
            TabAction::Show => {
                self.views.list.active = None;
                self.selected = Some(id.into());
            }
            TabAction::KeepOpen => self.tabs.pin(active, &id),
            TabAction::Close => {
                let neighbor = self.tabs.close(active, &id);
                if id == current {
                    self.selected = neighbor.map(Rc::from);
                    self.pane.opened = None; // its editor state went with the tab
                }
            }
            TabAction::CloseOthers => {
                self.tabs.close_others(active, &id);
                self.selected = Some(id.into());
            }
            TabAction::CloseToTheRight => {
                self.tabs.close_to_the_right(active, &id);
                if !self.tabs.of(active).any(|tab| tab.id == current) {
                    self.selected = Some(id.into());
                }
            }
            TabAction::CloseAll => self.close_all_tabs(),
        }
    }

    /// What a diff's / blame's tab asked for: as an object's, by its index in the views.
    fn view_tab_action(&mut self, action: TabAction, at: usize) {
        match action {
            TabAction::Show => {
                if let Some(view) = self.views.list.get(at) {
                    self.selected = Some(view.id.as_str().into());
                    self.views.show(at);
                }
            }
            TabAction::KeepOpen => self.views.list.pin(at),
            TabAction::Close => _ = self.views.list.close(at),
            // Every other tab of the behavior, objects' and views'.
            TabAction::CloseOthers => {
                self.tabs.close_all(self.active);
                self.views.close_others(at);
                if let Some(view) = self.views.list.active.and_then(|shown| self.views.list.get(shown)) {
                    self.selected = Some(view.id.as_str().into());
                }
            }
            TabAction::CloseToTheRight => self.views.list.close_to_the_right(at),
            TabAction::CloseAll => self.close_all_tabs(),
        }
    }

    /// Closes every tab of the behavior, objects' and views'.
    fn close_all_tabs(&mut self) {
        self.tabs.close_all(self.active);
        self.views.list.close_all(self.active);
        self.selected = None;
        self.pane.opened = None;
    }

    /// Does what the editor asked for. Returns whether the keyboard goes to the object tree.
    pub(in crate::patch) fn apply_editor_result(&mut self, result: EditorResult, id: &str) -> bool {
        if result.changed {
            self.workspace.behaviors[self.active].set_text(id, &self.pane.text);
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
            Some(EditorCommand::Save) => self.autosave.save_soon(),
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
            Some(EditorCommand::NextChange) => self.step_change(true),
            Some(EditorCommand::RevealFile) => {
                self.file_asked = Some((crate::patch::tool::list::FileAction::Reveal, id.to_owned()));
            }
            Some(EditorCommand::PreviousChange) => self.step_change(false),
            Some(EditorCommand::ToggleExplorer | EditorCommand::GoToDefinition | EditorCommand::FindReferences)
            | None => {}
        }
        match result.configure {
            // The palette's "Open Keyboard Shortcuts (JSON)" comes the same way.
            Some(action) if self.key_ui.run_action(action) => {}
            Some(command) => self.key_ui.configure_command(command),
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

    /// Vim `]c` / `[c`: to the next or previous changed place of what shows the changes, as its `▲ ▼` do.
    /// The inline diff first, as it is where the cursor is. e.g. changes at lines 3 and 9: from 5, `]c` is 9.
    pub(in crate::patch) fn step_change(&mut self, forward: bool) {
        if self.pane.show_inline_diff {
            let line = self.pane.inline_diff.step(self.pane.cursor_line, forward);
            if line.is_some() {
                self.pane.focus = line.map(FocusAt::Line);
            }
        } else if self.pane.show_preview
            && let Some(view) = &mut self.pane.preview_view
        {
            view.step(forward);
        }
    }

    /// `↶` in the inline diff: puts a changed place back to the template.
    pub(in crate::patch) fn revert_inline(&mut self, id: &str, index: usize) {
        let behavior = &mut self.workspace.behaviors[self.active];
        let Some(template) = behavior.template_object(id) else { return };
        self.pane.text = InlineDiff::revert(&template.text, &self.pane.text, index);
        behavior.set_text(id, &self.pane.text);
        self.pane.preview = None;
        self.object_changed(id);
        self.note_pandora_edit(id);
    }

    /// `↶` in the preview: puts changed block `block` back to the template.
    pub(in crate::patch) fn revert_block(&mut self, id: &str, block: usize) {
        let is_pandora = self.pane.preview_view.as_ref().is_some_and(editor::preview::Preview::is_pandora);
        let behavior = &mut self.workspace.behaviors[self.active];
        let Some(template) = behavior.template_object(id) else { return };
        if is_pandora {
            let opened = nemesis_diff::open_kept_arrays(&self.pane.text);
            let Some(text) = pandora_patch::doc::without_op(&template.text, &opened, block) else { return };
            self.pane.text = text;
        } else {
            self.pane.text = nemesis_diff::revert_hunk(&template.text, &self.pane.text, block);
        }
        behavior.set_text(id, &self.pane.text);
        self.pane.preview = None;
        self.object_changed(id);
        self.note_pandora_edit(id);
    }

    /// Object `id` was edited here: its part of the Pandora patch file follows at the next pause.
    fn note_pandora_edit(&mut self, id: &str) {
        if self.workspace.behaviors[self.active].pandora_doc.is_some()
            && !self.pandora_edited.iter().any(|edited| edited == id)
        {
            self.pandora_edited.push(id.to_owned());
        }
    }

    /// What was edited since the last time goes to the other side: an object to its part of the Pandora
    /// patch file, the file to the objects. After a pause in typing, as it reads the whole file.
    pub(in crate::patch) fn sync_pandora_doc(&mut self) {
        let edited = std::mem::take(&mut self.pandora_edited);
        let active = self.active;
        for id in edited {
            let Some(behavior) = self.workspace.behaviors.get_mut(active) else { continue };
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
        self.names.changed(id);
        self.mark_dirty();
    }

    /// The behaviors themselves changed (loaded, closed): `names` starts over.
    pub(in crate::patch) fn reset_names(&mut self) {
        self.names.reset();
        self.edits += 1; // what was found of the work before is not of this one
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
        tool.workspace.behaviors.push(clip_data(&["M~1/a.txt"]));
        tool.selected = Some("M~1/a.txt".into());
        let place = tool.editor_place().unwrap();

        tool.selected = None;
        tool.reopen_editor_place(&place);
        assert_eq!(tool.selected.as_deref(), Some("M~1/a.txt"));
        assert!(matches!(tool.pane.focus, Some(FocusAt::Keep)));

        // Gone from the mod: nothing is opened.
        tool.workspace.behaviors[0] = clip_data(&[]);
        tool.selected = None;
        tool.reopen_editor_place(&place);
        assert_eq!(tool.selected, None);
    }
}
