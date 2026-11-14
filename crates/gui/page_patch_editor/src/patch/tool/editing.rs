//! The open object: its tabs, what it shows beside the text, and what the editor asked for.

use std::{path::Path, rc::Rc, time::Instant};

use egui_code_editor::{CodeEditorState, Diagnostic, EditorCommand, InlineDiff};
use egui_widgets::TabAction;
use gui_shared::settings::ui::editor::{EditorMode, LspOptions};

use super::{
    keys::Keys,
    list::FileAction,
    selection::Selection,
    session::{Reshow, Shown},
    tab_events,
    work_state::Work,
};
use crate::patch::{
    EditorResult, FocusAt, TabEvent, anim,
    editor::{self, tabs::EditorTabs},
    problems::Problems,
    ui::revision_view::Views,
    workspace::{Behavior, PANDORA_DOC},
};

/// The editor of the open object, and what is shown beside it.
///
/// ```text
/// [tabs]                                             (EditorArea::tabs)
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
    /// The editor had the keyboard last frame. See [`Self::note_focus`].
    pub(in crate::patch) had_focus: bool,
    /// Where the keyboard goes next frame.
    pub(in crate::patch) focus: Option<FocusAt>,
    /// The line of the cursor, last frame.
    pub(in crate::patch) cursor_line: usize,
    /// The patch of the open object and the parser's word on it. `None`: to compute again, see
    /// [`EditorArea::refresh_preview`].
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

/// The editor area: the open object's editor, its tabs, and the diffs / blames shown as tabs.
///
/// ```text
/// [tabs: objects | views]
/// ┌ pane: the open object, its preview / merge ┐
/// └────────────────────────────────────────────┘
/// ```
/// What is shown is the [`Selection`]: the area follows it, and changes it (a tab, back, a link).
#[derive(Debug, Default)]
pub(in crate::patch) struct EditorArea {
    pub(in crate::patch) pane: EditorPane,
    /// The objects open in tabs.
    pub(in crate::patch) tabs: EditorTabs,
    /// The diffs and blames of the history open as tabs. See [`Views`].
    pub(in crate::patch) views: Views,
}

impl EditorArea {
    /// What is shown, for the work to write with itself.
    pub(in crate::patch) const fn shown<'a>(&'a self, sel: &'a Selection) -> Shown<'a> {
        Shown { tabs: &self.tabs, sel }
    }

    /// The work changed as a whole: the tabs of before close, and `reshow` is shown.
    pub(in crate::patch) fn reshow(&mut self, reshow: Reshow, work: &mut Work, sel: &mut Selection) {
        self.tabs.clear();
        self.pane.opened = None;
        self.pane.preview = None;
        *sel = Selection::default();

        match reshow {
            Reshow::Nothing | Reshow::Place(None) => {}
            Reshow::Session(session) => {
                let restored = super::session::restore(session, &work.workspace.behaviors, &mut self.tabs);
                (sel.active, sel.selected) = (restored.active, restored.selected);
                work.take_restored_place((restored.mod_dir, restored.output));
                work.autosave.seen_at(sel.active, sel.selected.as_ref());
            }
            Reshow::Place(Some(place)) => self.reopen_place(&work.workspace.behaviors, sel, &place),
        }
    }

    /// Shows `(meshes_path, id)` again, if `behaviors` still has it, with the keyboard in the editor
    /// at its cursor.
    fn reopen_place(&mut self, behaviors: &[Behavior], sel: &mut Selection, (meshes_path, id): &(String, Rc<str>)) {
        let found = behaviors.iter().position(|behavior| behavior.meshes_path == *meshes_path);
        let Some(index) = found.filter(|&index| behaviors[index].text(id).is_some()) else { return };

        sel.active = index;
        self.tabs.open(index, id, None);
        sel.selected = Some(Rc::clone(id));
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// The next / previous tab, the views' after the objects'. `current`: the object shown, if no
    /// view is. See [`tab_events::step`].
    pub(in crate::patch) fn step_tab(&mut self, sel: &mut Selection, current: &str, forward: bool) {
        let outcome = tab_events::step((&self.tabs, &mut self.views), sel.active, current, forward);
        self.apply_tab_outcome(sel, outcome);
    }

    /// Vim `Ctrl+O` / `Ctrl+I`: the previous / next visited object of `behavior` (skips deleted ones).
    pub(in crate::patch) fn walk_history(
        &mut self,
        (behavior, sel): (&Behavior, &mut Selection),
        older: bool,
        current: &str,
    ) {
        let mut current = current.to_owned();
        while let Some(target) = self.pane.history.walk(older, &current) {
            if behavior.text(&target).is_some() {
                sel.selected = Some(target.into());
                self.pane.focus = Some(FocusAt::Keep);
                return;
            }
            current = target;
        }
    }

    /// Opens `document` at `line` (go to definition, a click in the references window or the
    /// search).
    pub(in crate::patch) fn open_location(
        &mut self,
        sel: &mut Selection,
        current: &str,
        (document, line): (&str, usize),
    ) {
        if document != current {
            self.pane.history.visit(current);
            sel.selected = Some(document.into());
        }
        self.pane.focus = Some(FocusAt::Line(line));
    }

    /// Double click or menu in the tree and the list: shows `id` in a tab that stays.
    pub(in crate::patch) fn keep_open(&mut self, sel: &mut Selection, id: &str) {
        let current = sel.selected.as_deref().filter(|selected| *selected != id);
        self.tabs.open(sel.active, id, current);
        self.tabs.pin(sel.active, id);
        sel.selected = Some(id.into());
    }

    /// Shows the Pandora patch file of the active behavior (see [`Work::make_pandora_doc`]).
    pub(in crate::patch) fn open_pandora_doc(&mut self, sel: &mut Selection) {
        if let Some(current) = sel.selected.as_deref().filter(|id| *id != PANDORA_DOC) {
            self.pane.history.visit(current);
        }
        sel.selected = Some(PANDORA_DOC.into());
    }

    /// Object `id` of the active behavior was reverted, or is gone (`gone`: its tab closes, and its
    /// neighbor is shown if it was).
    pub(in crate::patch) fn object_reverted(&mut self, sel: &mut Selection, id: &str, gone: bool) {
        if gone {
            let neighbor = self.tabs.close(sel.active, id);
            if sel.selected.as_deref() == Some(id) {
                sel.selected = neighbor.map(Rc::from);
            }
        }
        self.pane.opened = None;
    }

    /// The objects changed under the editor: it loads the open one again.
    pub(in crate::patch) fn reload(&mut self) {
        self.pane.opened = None;
        self.pane.preview = None;
    }

    /// The behaviors changed (one removed): the numbers of the others shift, so the tabs close.
    pub(in crate::patch) fn behaviors_changed(&mut self) {
        self.tabs.clear();
        self.pane.opened = None;
    }

    /// Behavior `at` is shown: the object it showed last, if any.
    pub(in crate::patch) fn show_behavior(&self, sel: &mut Selection, at: usize) {
        sel.active = at;
        sel.selected = self.tabs.last_of(at).map(Rc::from);
    }

    /// Loads `id` of behavior `active` into the editor if it is not the open object already.
    pub(in crate::patch) fn open_in_editor(&mut self, work: &Work, active: usize, id: &str) {
        if self.pane.opened.as_ref().is_some_and(|(open_in, open)| *open_in == active && open == id) {
            return;
        }

        // The tab being left keeps its undo history and Vim state.
        let left = self.pane.opened.take();
        if let Some((behavior, left)) = &left {
            self.tabs.store_state(*behavior, left, std::mem::take(&mut self.pane.state));
        }
        let current = left.as_ref().filter(|(behavior, _)| *behavior == active).map(|(_, id)| id.as_str());
        self.tabs.open(active, id, current);
        self.pane.state = self.tabs.take_state(active, id);

        let behavior = &work.workspace.behaviors[active];
        self.pane.refresh_trigger_names(behavior, Path::new(&work.place.template_dir));
        behavior.text(id).unwrap_or_default().clone_into(&mut self.pane.text);
        self.pane.preview = None;
        self.pane.opened = Some((active, id.to_owned()));
    }

    /// Recomputes the preview of `id` of behavior `active` if it is due, and then the work's problems
    /// of it. See [`EditorPane::refresh_preview`].
    pub(in crate::patch) fn refresh_preview(
        &mut self,
        (work, problems): (&mut Work, &mut Problems),
        active: usize,
        id: &str,
    ) {
        if self.pane.preview.is_some() {
            return;
        }
        work.sync_pandora_doc(active);
        let settings = super::pane::PreviewSettings {
            code: work.place.mod_code.trim(),
            parser_mode: work.parser_mode,
            export_format: work.workspace.export_format,
        };
        if self.pane.refresh_preview(&work.workspace.behaviors[active], id, settings) {
            problems.refresh_in(work, active, id);
        }
    }

    /// Does what the tab bar asked for. `current`: the object shown now. See [`tab_events`].
    /// Returns what is asked of an object's file, done where the notifications are.
    pub(in crate::patch) fn apply_tab_event(
        &mut self,
        sel: &mut Selection,
        event: Option<TabEvent>,
        current: &str,
    ) -> Option<(FileAction, String)> {
        let tabs = (&mut self.tabs, &mut self.views);
        let outcome = match event? {
            TabEvent::Object(action, id) => tab_events::object_action(tabs, sel.active, (action, id), current),
            TabEvent::View(action, at) => tab_events::view_action(tabs, sel.active, (action, at)),
            TabEvent::File(action, id) => return Some((action, id)),
        };
        self.apply_tab_outcome(sel, outcome);
        None
    }

    /// What follows a tab event: another selection, the editor's state gone.
    fn apply_tab_outcome(&mut self, sel: &mut Selection, outcome: tab_events::TabOutcome) {
        match outcome.select {
            tab_events::Select::Keep => {}
            tab_events::Select::Object(id) => sel.selected = Some(id),
            tab_events::Select::Nothing => sel.selected = None,
        }
        if outcome.close_editor {
            self.pane.opened = None;
        }
        if outcome.keep_focus {
            self.pane.focus = Some(FocusAt::Keep);
        }
    }

    /// Does what the editor asked for of object `id`. See [`EditorDone`].
    pub(in crate::patch) fn apply_editor_result(
        &mut self,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
        result: EditorResult,
        id: &str,
    ) -> EditorDone {
        if result.changed {
            work.workspace.behaviors[sel.active].set_text(id, &self.pane.text);
            self.pane.typed_at = Some(Instant::now());
            self.tabs.pin(sel.active, id); // edited: it stays open
            work.object_changed(id);
            work.note_pandora_edit(sel.active, id);
        }
        if let Some(line) = result.jump_to_line {
            self.pane.focus = Some(FocusAt::Line(line));
        }
        let mut file = self.apply_tab_event(sel, result.tab_event, id);
        if let Some(index) = result.inline_revert {
            self.revert_inline((work, sel.active), id, index);
        }

        let mut walk = result.walk_history;
        match result.command {
            // Vim `:w`: save the work now.
            Some(EditorCommand::Save) => work.autosave.save_soon(),
            Some(EditorCommand::JumpBack) => walk = Some(true),
            Some(EditorCommand::JumpForward) => walk = Some(false),
            Some(EditorCommand::NextTab) => self.step_tab(sel, id, true),
            Some(EditorCommand::PreviousTab) => self.step_tab(sel, id, false),
            Some(EditorCommand::CloseTab) => {
                self.apply_tab_event(sel, Some(TabEvent::Object(TabAction::Close, id.to_owned())), id);
                self.pane.focus = Some(FocusAt::Keep);
            }
            Some(EditorCommand::TogglePreview) => self.pane.show_preview = !self.pane.show_preview,
            Some(EditorCommand::ToggleInlineDiff) => self.pane.show_inline_diff = !self.pane.show_inline_diff,
            Some(EditorCommand::NextChange) => self.pane.step_change(true),
            Some(EditorCommand::PreviousChange) => self.pane.step_change(false),
            Some(EditorCommand::RevealFile) => file = Some((FileAction::Reveal, id.to_owned())),
            Some(EditorCommand::ToggleExplorer | EditorCommand::GoToDefinition | EditorCommand::FindReferences)
            | None => {}
        }

        match result.configure {
            // The palette's "Open Keyboard Shortcuts (JSON)" comes the same way.
            Some(action) if keys.ui.run_action(action) => {}
            Some(command) => keys.ui.configure_command(command),
            None => {}
        }
        if let Some(older) = walk {
            self.walk_history((&work.workspace.behaviors[sel.active], sel), older, id);
        }
        if let Some((document, line)) = result.definition {
            self.open_location(sel, id, (&document, line));
        }

        let focus_tree = result.focus_tree || result.command == Some(EditorCommand::ToggleExplorer);
        EditorDone { focus_tree, file }
    }

    /// `↶` in the inline diff: puts a changed place of object `id` back to the template.
    fn revert_inline(&mut self, (work, active): (&mut Work, usize), id: &str, index: usize) {
        let Some(template) = work.workspace.behaviors[active].template_object(id) else { return };
        let text = InlineDiff::revert(&template.text, &self.pane.text, index);
        self.replace_object_text((work, active), id, text);
    }

    /// `↶` in the preview: puts changed block `block` of object `id` back to the template.
    pub(in crate::patch) fn revert_block(&mut self, (work, active): (&mut Work, usize), id: &str, block: usize) {
        let Some(template) = work.workspace.behaviors[active].template_object(id) else { return };
        let Some(text) = self.pane.reverted_block(&template.text, block) else { return };
        self.replace_object_text((work, active), id, text);
    }

    /// Object `id` is `text` now, in the editor and in the work.
    fn replace_object_text(&mut self, (work, active): (&mut Work, usize), id: &str, text: String) {
        self.pane.text = text;
        work.workspace.behaviors[active].set_text(id, &self.pane.text);
        self.pane.preview = None;
        work.object_changed(id);
        work.note_pandora_edit(active, id);
    }
}

/// What is left to do after [`EditorArea::apply_editor_result`], by others.
#[derive(Debug, Default)]
pub(in crate::patch) struct EditorDone {
    /// The keyboard goes to the object tree.
    pub(in crate::patch) focus_tree: bool,
    /// Copy the path of / reveal the file of an object.
    pub(in crate::patch) file: Option<(FileAction, String)>,
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
        let (mut area, mut work, mut sel) = (EditorArea::default(), Work::default(), Selection::default());
        work.workspace.behaviors.push(clip_data(&["M~1/a.txt"]));
        let place = (work.workspace.behaviors[0].meshes_path.clone(), Rc::from("M~1/a.txt"));

        area.reshow(Reshow::Place(Some(place.clone())), &mut work, &mut sel);
        assert_eq!(sel.selected.as_deref(), Some("M~1/a.txt"));
        assert!(matches!(area.pane.focus, Some(FocusAt::Keep)));

        // Gone from the mod: nothing is opened.
        work.workspace.behaviors[0] = clip_data(&[]);
        area.reshow(Reshow::Place(Some(place)), &mut work, &mut sel);
        assert_eq!(sel.selected, None);
    }
}
