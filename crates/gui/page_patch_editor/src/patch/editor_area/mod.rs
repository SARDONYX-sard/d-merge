//! The open object: its tabs, what it shows beside the text, and what the editor asked for.

use std::{path::Path, rc::Rc, time::Instant};

use egui_code_editor::{EditorCommand, InlineDiff};
use egui_widgets::TabAction;

mod pane;
mod revision_view;
mod tab_bar;
mod tab_events;
mod view;

use gui_shared::settings::{pages::patch_editor::PatchEditorSettings, ui::editor::LspOptions};

pub(super) use self::view::EditorOut;
use self::{pane::EditorPane, revision_view::Views};
use crate::patch::{
    EditorResult, FocusAt, TabEvent,
    editor::{self, tabs::EditorTabs},
    problems::Problems,
    tool::{
        file::FileAction,
        keys::Keys,
        selection::Selection,
        session::{Reshow, Shown},
        work_state::Work,
    },
    workspace::{Behavior, PANDORA_DOC},
};

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
    pane: EditorPane,
    /// The objects open in tabs.
    tabs: EditorTabs,
    /// The diffs and blames of the history open as tabs. See [`Views`].
    views: Views,
}

impl EditorArea {
    /// What is shown, for the work to write with itself.
    pub(in crate::patch) const fn shown<'a>(&'a self, sel: &'a Selection) -> Shown<'a> {
        Shown::new(&self.tabs, sel)
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
                (sel.active, sel.selected) = work.restore_session(session, &mut self.tabs);
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
    fn walk_history(&mut self, (behavior, sel): (&Behavior, &mut Selection), older: bool, current: &str) {
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

    /// The editor's options, as the settings have them.
    pub(in crate::patch) fn read_options(&mut self, options: &LspOptions) {
        self.pane.read_options(options);
    }

    pub(in crate::patch) fn write_options(&mut self, options: &mut LspOptions) {
        self.pane.write_options(options);
    }

    /// What is shown beside the editor (the preview, the inline diff), as the settings have it.
    pub(in crate::patch) const fn read_view(&mut self, settings: &PatchEditorSettings) {
        self.pane.read_view(settings);
    }

    pub(in crate::patch) const fn write_view(&self, settings: &mut PatchEditorSettings) {
        self.pane.write_view(settings);
    }

    /// What the key bindings see of the editor: `(it has the keyboard, Vim, Vim's mode)`.
    pub(in crate::patch) const fn key_state(&self) -> (bool, bool, Option<&'static str>) {
        let keymap = egui_code_editor::Keymap::from_vim(self.pane.mode.is_vim());
        (self.pane.had_focus, keymap.is_vim(), self.pane.state.keymap.vim_mode(keymap))
    }

    /// The keyboard goes back to the editor, where its cursor was.
    pub(in crate::patch) const fn focus_editor(&mut self) {
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// The preview is computed again (another parser mode): what it says changed.
    pub(in crate::patch) fn preview_again(&mut self) {
        self.pane.preview = None;
    }

    /// A conflict's `values` beside the object it went to, with the lines `accepted` already; the
    /// cursor at `line`. See [`crate::patch::editor::merge`].
    pub(in crate::patch) fn show_merge(
        &mut self,
        values: Option<editor::resolve::MergeValues>,
        accepted: Vec<String>,
        line: Option<usize>,
    ) {
        self.pane.opened = None;
        self.pane.merge_values = values;
        self.pane.merge_accepted = accepted;
        self.pane.focus = line.map(FocusAt::Line);
    }

    /// The conflict's values are no longer shown.
    pub(in crate::patch) fn close_merge(&mut self) {
        self.pane.merge_values = None;
    }

    /// The open object and its text, for the tests.
    #[cfg(test)]
    pub(in crate::patch) fn opened(&self) -> Option<(&str, &str)> {
        let (_, id) = self.pane.opened.as_ref()?;
        Some((id, &self.pane.text))
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
    fn refresh_preview(&mut self, (work, problems): (&mut Work, &mut Problems), active: usize, id: &str) {
        if self.pane.preview.is_some() {
            return;
        }
        work.sync_pandora_doc(active);
        let settings = pane::PreviewSettings {
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
    fn apply_tab_event(
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
    fn apply_editor_result(
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
struct EditorDone {
    /// The keyboard goes to the object tree.
    focus_tree: bool,
    /// Copy the path of / reveal the file of an object.
    pub(in crate::patch) file: Option<(FileAction, String)>,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::patch::workspace::{ADSF, Behavior, ExportFormat, Workspace};

    /// A clip data "behavior" with the files `ids` (it needs no template file).
    fn clip_data(ids: &[&str]) -> Behavior {
        let mut adsf = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        for id in ids {
            adsf.added.insert((*id).to_owned(), String::new());
        }
        adsf
    }

    /// The preview says what `Export` would write, and its `↶` takes back what it shows.
    #[test]
    fn the_preview_follows_the_export_format() {
        let dir = std::env::temp_dir().join(format!("d_merge_pandora_preview_{}", std::process::id()));
        let meshes = dir.join("templates/meshes/actors/character/behaviors");
        std::fs::create_dir_all(&meshes).unwrap();
        std::fs::write(
            meshes.join("0_master.xml"),
            "<hkpackfile><hksection name=\"__data__\">\n<hkobject name=\"#0001\" class=\"hkbStateMachine\" signature=\"0x1\">\n\t<hkparam name=\"enable\">true</hkparam>\n</hkobject>\n</hksection></hkpackfile>\n",
        )
        .unwrap();
        let mod_dir = dir.join("mods/Pandora/Pandora_Engine/mod/pandora");
        std::fs::create_dir_all(mod_dir.join("patches")).unwrap();
        std::fs::write(
            mod_dir.join("patches/0_master.xml"),
            "<patch>\n\t<Replace path=\"#0001/enable\">\n\t\t<hkparam name=\"enable\">false</hkparam>\n\t</Replace>\n</patch>\n",
        )
        .unwrap();
        let (workspace, code, _) = Workspace::import(&mod_dir, &dir.join("templates")).unwrap();
        let (mut area, mut work, mut problems) = (EditorArea::default(), Work::default(), Problems::default());
        (work.workspace, work.place.mod_code) = (workspace, code);
        area.pane.text = work.workspace.behaviors[0].text("#0001").unwrap().to_owned();

        let mut shown = |work: &mut Work| {
            area.pane.preview = None;
            area.refresh_preview((work, &mut problems), 0, "#0001");
            let view = area.pane.preview_view.as_ref();
            let lines = view.map(|view| view.lines().iter().map(|(_, line)| line.clone()));
            lines.map_or_else(String::new, |lines| lines.collect::<Vec<_>>().join("\n"))
        };
        let as_pandora = shown(&mut work);
        work.workspace.export_format = ExportFormat::Nemesis;
        let as_nemesis = shown(&mut work);
        work.workspace.export_format = ExportFormat::PandoraV5;
        shown(&mut work);
        area.revert_block((&mut work, 0), "#0001", 0);
        let edits = work.workspace.behaviors[0].edits.len();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(as_pandora.starts_with("<Replace path=\"#0001/enable\">"), "{as_pandora}");
        assert!(as_nemesis.contains("<!-- MOD_CODE ~"), "{as_nemesis}");
        assert_eq!(edits, 0, "back to the template");
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
