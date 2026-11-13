//! The OBJECTS section of the explorer, below the behaviors: the objects of the active one.
//!
//! ```text
//! ▼ OBJECTS          [+] [≡] [⋯]    <- new, tree or flat list, the rest (see `more_menu`)
//! [filter.....................]
//! (the tree or the list)
//! ```
//! The title's actions are the explorer's: what is done to one object is in its right-click menu.
//! A window has the class list: nothing moves in the side bar when it is opened.
mod anim_list;
mod flat;
mod title;

use egui_widgets::{
    notify::Notification,
    shadcn::{Button, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;

use super::{
    anim_data::{DuplicateForm, NewClipsForm},
    anim_tree::{self, AnimFile, AnimRow},
};
use crate::patch::{
    PatchTool,
    anim::data,
    tool::list::take_reveal,
    tree::{Graph, Node, TreeState},
    workspace::PANDORA_DOC,
};

impl PatchTool {
    pub(in crate::patch) fn object_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let anim = self.workspace.behaviors[self.active].is_anim_data();
        let mut open = !self.side.objects_closed;
        let asked = {
            let behavior = &self.workspace.behaviors[self.active];
            let selected = self.selected.as_ref();
            let title = title::Title {
                anim,
                adsf: behavior.adsf,
                tree_view: self.explorer.tree_view,
                code_ok: is_existing_mod_code(self.place.mod_code.trim()),
                selected,
                selected_is_new: selected.is_some_and(|id| **id == *PANDORA_DOC || behavior.added.contains_key(&**id)),
                selected_is_pandora_doc: selected.is_some_and(|id| **id == *PANDORA_DOC),
                changed_only: &mut self.explorer.changed_only,
                unlinked_only: &mut self.explorer.unlinked_only,
            };
            let mut asked = None;
            egui_widgets::section_header(ui, &mut open, i18n.t(I18nKey::NemesisObjectsTab), |ui| {
                asked = title::buttons(ui, i18n, title);
            });
            asked
        };
        if let Some(asked) = asked {
            self.apply_title(asked);
        }
        self.side.objects_closed = !open;
        if self.side.objects_closed {
            return;
        }
        ui.add(
            TextEdit::singleline(&mut self.explorer.filter)
                .hint_text(i18n.t(I18nKey::NemesisFilterHint))
                .desired_width(ui.available_width()),
        );

        // Clicking any text field takes the keyboard out of the tree.
        if ui.memory(|m| m.focused().is_some()) {
            self.explorer.tree_focus = false;
        }
        // `t` / `c` / `u` (key bindings): what it shows, as the `[≡]` / `⋯` menu does.
        self.explorer.tree_view ^= self.key_command(keybinding::commands::EXPLORER_TOGGLE_VIEW) && !anim;
        self.explorer.changed_only ^= self.key_command(keybinding::commands::EXPLORER_TOGGLE_CHANGED_ONLY);
        self.explorer.unlinked_only ^= self.key_command(keybinding::commands::EXPLORER_TOGGLE_UNLINKED_ONLY) && !anim;
        let names = self.names();
        // A tree from the root container, unless filtering: a filter wants the flat list.
        if anim {
            self.anim_tree_panel(ui, i18n, &names.graph, notify);
        } else if self.explorer.tree_view && self.explorer.filter.is_empty() && !self.explorer.unlinked_only {
            self.tree_panel(ui, i18n, &names.graph, notify);
        } else {
            self.flat_list(ui, i18n, &names.graph, notify);
        }
    }

    /// What a button of the title asked for.
    fn apply_title(&mut self, asked: title::TitleAsk) {
        use title::TitleAsk;
        match asked {
            TitleAsk::ToggleTreeView => self.explorer.tree_view = !self.explorer.tree_view,
            TitleAsk::ExpandAll => self.anim_tree.expand_all = true,
            TitleAsk::CollapseAll => self.anim_tree.open.clear(),

            TitleAsk::OpenPandoraDoc => self.open_pandora_doc(),
            TitleAsk::Duplicate(id) => self.new_object(Some(&id)),
            TitleAsk::RevertOrDelete(id) => self.revert_or_delete(&id),

            TitleAsk::New if self.workspace.behaviors[self.active].adsf => {
                let projects = self.anim_projects();
                self.panels.new_clips = Some(NewClipsForm::new(self.selected.as_deref(), projects));
            }
            TitleAsk::New => {
                self.panels.class_picker.open();
            }
        }
    }

    /// The tree, and its keys while it has the keyboard.
    fn tree_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph, notify: &mut Notification) {
        let path = &self.workspace.behaviors[self.active].meshes_path;
        if !self.explorer.trees.contains_key(path) {
            self.explorer.trees.insert(path.clone(), TreeState::default());
        }
        let Some(tree) = self.explorer.trees.get_mut(path) else { return };
        if std::mem::take(&mut self.explorer.reveal) {
            tree.reveal_again();
        }
        if self.explorer.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let inputs = keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key);
            let source = super::super::tree::ObjectTree { graph, changed_only: self.explorer.changed_only };
            tree.prepare(&source, self.selected.as_deref());
            let keys = tree.apply_keys(&source, &inputs);
            if let Some(node) = keys.selected {
                self.selected = Some(graph.id(node).into());
            }
            if keys.leave {
                self.leave_tree();
                return;
            }
        }
        let flags = (self.explorer.changed_only, self.explorer.tree_focus);
        let action = super::tree::tree_ui(ui, graph, &mut self.selected, tree, flags, i18n);
        self.apply_row_action(ui, graph, action, notify);
    }

    /// The anim data files after the filter: the rows of [`anim_tree::rows`].
    fn anim_tree_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph, notify: &mut Notification) {
        self.refresh_list(graph);
        let is_adsf = self.workspace.behaviors[self.active].adsf;

        // Only when the editor gets the keyboard: else what is open, and where it is scrolled, is the user's.
        let forced = std::mem::take(&mut self.explorer.reveal);
        let reveal = take_reveal(self.selected.as_ref(), forced);
        if reveal && let Some(id) = &self.selected {
            anim_tree::reveal(&mut self.anim_tree.open, is_adsf, id);
        }

        let files: Vec<AnimFile<'_>> = {
            let nodes = self.explorer.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
            nodes.iter().map(|&node| AnimFile { node, id: graph.id(node), changed: graph.is_changed(node) }).collect()
        };
        if std::mem::take(&mut self.anim_tree.expand_all) {
            anim_tree::expand_all(&mut self.anim_tree.open, &files);
        }
        let filtering = !self.explorer.filter.is_empty();
        let rebuild = |state: &anim_tree::AnimTreeState| anim_tree::rows(&files, &state.open, is_adsf, filtering);
        let mut rows = rebuild(&self.anim_tree);

        let moved = self.anim_keys(ui, i18n, graph, &mut rows, &rebuild);
        let cursor = self.explorer.tree_focus.then_some(self.anim_tree.cursor);

        let projects = self.anim_projects();
        let (event, scroll) = {
            let selected_row =
                self.selected.as_deref().and_then(|id| graph.node_of(id)).and_then(|node| file_row(&rows, node));
            let input = anim_list::ListInput {
                graph,
                selected: self.selected.as_deref(),
                cursor,
                scroll_to: if moved { cursor } else { selected_row.filter(|_| reveal) },
                scroll: self.explorer.list_scroll,
                projects: &projects,
            };
            anim_list::show(ui, i18n, &rows, &input)
        };
        self.explorer.list_scroll = scroll;

        if let Some(event) = event {
            self.apply_anim_event(ui, graph, event, &projects, notify);
        }
        self.duplicate_anim_files();
    }

    /// The keys, while the list has the keyboard: a file the cursor gets to is selected. Else the
    /// cursor follows the selection. Returns whether the keys moved it.
    fn anim_keys<'a>(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        graph: &Graph,
        rows: &mut Vec<AnimRow<'a>>,
        rebuild: &impl Fn(&anim_tree::AnimTreeState) -> Vec<AnimRow<'a>>,
    ) -> bool {
        let mut moved = false;
        if self.explorer.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let keys = keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key);
            let done = anim_list::keys(&mut self.anim_tree, &keys, rows, rebuild);
            if done.leave {
                self.leave_tree();
            }
            moved = done.moved;
            if moved && let Some(AnimRow::File { node, .. }) = rows.get(self.anim_tree.cursor) {
                self.selected = Some(graph.id(*node).into());
            }
        } else if let Some(at) =
            self.selected.as_deref().and_then(|id| graph.node_of(id)).and_then(|node| file_row(rows, node))
        {
            self.anim_tree.cursor = at; // the keyboard starts where the selection is
        }

        self.anim_tree.cursor = self.anim_tree.cursor.min(rows.len().saturating_sub(1));
        moved
    }

    /// What a row of the list asked for.
    fn apply_anim_event(
        &mut self,
        ui: &egui::Ui,
        graph: &Graph,
        event: anim_list::AnimEvent,
        projects: &[String],
        notify: &mut Notification,
    ) {
        use anim_list::{AnimEvent, DuplicatePick, Files};
        match event {
            AnimEvent::Toggle(key) => anim_tree::toggle(&mut self.anim_tree.open, &key),
            AnimEvent::ExpandBelow(folder) => anim_tree::expand(&mut self.anim_tree.open, &folder),
            AnimEvent::CollapseBelow(folder) => anim_tree::collapse(&mut self.anim_tree.open, &folder),

            AnimEvent::Select(node) => self.selected = Some(graph.id(node).into()),
            AnimEvent::Menu(node, action) => self.apply_row_action(ui, graph, Some((node, action)), notify),

            AnimEvent::Duplicate { pick, files, from } => {
                let behavior = &self.workspace.behaviors[self.active];
                let files = match files {
                    Files::ChangedOf(folder) => data::changed_files(behavior, &folder),
                    Files::WithMotion(id) => data::with_motion(behavior, &id),
                };
                match pick {
                    DuplicatePick::To(to) => self.panels.pending_duplicate = Some((files, to)),
                    DuplicatePick::Choose => {
                        self.panels.duplicate_form = Some(DuplicateForm::new(files, projects.to_vec(), &from));
                    }
                }
            }
        }
    }

    /// The files go to the other project, which opens to show them.
    fn duplicate_anim_files(&mut self) {
        let Some((files, to)) = self.panels.pending_duplicate.take() else { return };
        let made = data::duplicate_to(&mut self.workspace.behaviors[self.active], &files, &to);
        for id in &made {
            self.object_changed(id);
        }
        self.anim_tree.open.insert(to);
    }

    /// The flat list after the filter (only the rows on screen are laid out).
    fn flat_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph, notify: &mut Notification) {
        self.refresh_list(graph);
        if self.explorer.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            self.list_keys(graph);
        }

        let rows = self.explorer.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        // Only when the editor gets the keyboard: else where it is scrolled is the user's.
        let forced = std::mem::take(&mut self.explorer.reveal);
        if take_reveal(self.selected.as_ref(), forced) {
            let selected = self.selected.as_deref().and_then(|id| graph.node_of(id));
            self.explorer.list_scroll_to = selected.and_then(|node| rows.iter().position(|&row| row == node));
        }

        let scroll = (self.explorer.list_scroll_to.take(), self.explorer.list_scroll);
        let (event, scroll) = flat::show(ui, i18n, (graph, rows), self.selected.as_deref(), scroll);
        self.explorer.list_scroll = scroll;

        match event {
            Some(flat::FlatEvent::Select(node)) => self.selected = Some(graph.id(node).into()),
            Some(flat::FlatEvent::Menu(node, action)) => self.apply_row_action(ui, graph, Some((node, action)), notify),
            None => {}
        }
    }

    fn apply_row_action(
        &mut self,
        ui: &egui::Ui,
        graph: &Graph,
        action: Option<(Node, RowAction)>,
        notify: &mut Notification,
    ) {
        let Some((node, action)) = action else { return };
        let id = graph.id(node);
        match action {
            RowAction::KeepOpen => self.keep_open(id),
            RowAction::NewObject => {
                self.panels.class_picker.open();
            }
            RowAction::Duplicate => self.new_object(Some(id)),
            RowAction::Repeat => {
                let behavior = &self.workspace.behaviors[self.active];
                self.panels.repeat = super::repeat::RepeatForm::of(self.active, behavior, id);
            }
            RowAction::CopyId => ui.ctx().copy_text(id.to_owned()),
            RowAction::RevertOrDelete => self.revert_or_delete(id),
            RowAction::CopyPath => self.copy_object_path(ui.ctx(), id),
            RowAction::Reveal => self.reveal_object(id, notify),
        }
    }
}

/// The row of file `node`, if it is shown.
fn file_row(rows: &[AnimRow<'_>], node: Node) -> Option<usize> {
    rows.iter().position(|row| matches!(row, AnimRow::File { node: n, .. } if *n == node))
}

/// What a row's right-click menu asked for, in the flat list and the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowAction {
    KeepOpen,
    /// A new object of a class to pick, as `[+]`.
    NewObject,
    Duplicate,
    /// Many new objects from this one: see [`crate::patch::repeat`].
    Repeat,
    CopyId,
    /// The path of the file the object is in: see [`PatchTool::object_file`].
    CopyPath,
    /// The file selected in the file manager: see [`PatchTool::reveal_object`].
    Reveal,
    RevertOrDelete,
}

/// "Reveal in File Explorer" as VS Code names it on this OS.
pub(super) const REVEAL_LABEL: I18nKey = gui_shared::i18n::REVEAL_IN_FILE_MANAGER;

/// `Keep Open / New / Duplicate / Repeat / Copy id / Copy path / Reveal in File Explorer / Revert (or Delete for a new object)`, for the flat list and
/// the tree alike.
pub(super) fn row_menu(ui: &mut egui::Ui, i18n: &I18nMap, is_new: bool, changed: bool) -> Option<RowAction> {
    let keep_open = ui.button(i18n.t(I18nKey::NemesisKeepOpen)).clicked();
    ui.separator();
    let new = ui.button(i18n.t(I18nKey::NemesisNewObject)).clicked();
    let duplicate = ui.button(i18n.t(I18nKey::NemesisDuplicate)).clicked();
    let repeat = ui.button(i18n.t(I18nKey::RepeatTitle)).on_hover_text(i18n.t(I18nKey::RepeatHover)).clicked();
    let copy = ui.button(i18n.t(I18nKey::NemesisCopyId)).clicked();
    let hover = i18n.t(I18nKey::NemesisObjectFileHover);
    let copy_path = ui.button(i18n.t(I18nKey::NemesisCopyPath)).on_hover_text(hover).clicked();
    let reveal = ui.button(i18n.t(REVEAL_LABEL)).on_hover_text(hover).clicked();
    let revert = changed && {
        ui.separator();
        let label = if is_new { I18nKey::NemesisDeleteObject } else { I18nKey::RevertButton };
        ui.add(Button::new(i18n.t(label)).destructive()).clicked()
    };
    [
        (keep_open, RowAction::KeepOpen),
        (new, RowAction::NewObject),
        (duplicate, RowAction::Duplicate),
        (repeat, RowAction::Repeat),
        (copy, RowAction::CopyId),
        (copy_path, RowAction::CopyPath),
        (reveal, RowAction::Reveal),
        (revert, RowAction::RevertOrDelete),
    ]
    .into_iter()
    .find_map(|(clicked, action)| clicked.then_some(action))
}
