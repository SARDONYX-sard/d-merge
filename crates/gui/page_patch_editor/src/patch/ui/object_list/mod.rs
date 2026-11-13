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
mod explorer;
mod flat;
mod title;

use egui_widgets::{
    notify::Notification,
    shadcn::{Button, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;

use self::{
    anim_list::{DuplicatePick, Files},
    explorer::ListOutput,
};
use super::anim_data::{DuplicateForm, NewClipsForm};
use crate::patch::{
    PatchTool,
    anim::data,
    tool::list::FileAction,
    tree::{Graph, Node},
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
        let graph = &names.graph;
        let keys = if self.explorer.tree_focus {
            keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key)
        } else {
            Vec::new()
        };

        // A tree from the root container, unless filtering: a filter wants the flat list.
        let output = if anim {
            let projects = self.anim_projects();
            let is_adsf = self.workspace.behaviors[self.active].adsf;
            self.explorer.anim(ui, i18n, (graph, is_adsf, &projects), &keys, &mut self.selected)
        } else if self.explorer.tree_view && self.explorer.filter.is_empty() && !self.explorer.unlinked_only {
            let path = &self.workspace.behaviors[self.active].meshes_path;
            self.explorer.tree(ui, i18n, (graph, path), &keys, &mut self.selected)
        } else {
            self.explorer.flat(ui, i18n, graph, &keys, &mut self.selected)
        };
        self.apply_list_output(ui, graph, output, notify);
    }

    /// What the list asked of the tool: back to the editor, a row's menu, a duplicate.
    fn apply_list_output(&mut self, ui: &egui::Ui, graph: &Graph, output: ListOutput, notify: &mut Notification) {
        if output.leave {
            self.leave_tree();
        }
        self.apply_row_action(ui, graph, output.menu, notify);

        if let Some((pick, files, from)) = output.duplicate {
            let behavior = &self.workspace.behaviors[self.active];
            let files = match files {
                Files::ChangedOf(folder) => data::changed_files(behavior, &folder),
                Files::WithMotion(id) => data::with_motion(behavior, &id),
            };
            match pick {
                DuplicatePick::To(to) => self.panels.pending_duplicate = Some((files, to)),
                DuplicatePick::Choose => {
                    let projects = self.anim_projects();
                    self.panels.duplicate_form = Some(DuplicateForm::new(files, projects, &from));
                }
            }
        }
        self.duplicate_anim_files();
    }

    /// What a button of the title asked for.
    fn apply_title(&mut self, asked: title::TitleAsk) {
        use title::TitleAsk;
        match asked {
            TitleAsk::ToggleTreeView => self.explorer.tree_view = !self.explorer.tree_view,
            TitleAsk::ExpandAll => self.explorer.anim_tree.expand_all = true,
            TitleAsk::CollapseAll => self.explorer.anim_tree.open.clear(),

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

    /// The files go to the other project, which opens to show them.
    fn duplicate_anim_files(&mut self) {
        let Some((files, to)) = self.panels.pending_duplicate.take() else { return };
        let made = data::duplicate_to(&mut self.workspace.behaviors[self.active], &files, &to);
        for id in &made {
            self.object_changed(id);
        }
        self.explorer.anim_tree.open.insert(to);
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
            RowAction::CopyPath => self.file_action(ui.ctx(), (FileAction::CopyPath, id), notify),
            RowAction::Reveal => self.file_action(ui.ctx(), (FileAction::Reveal, id), notify),
        }
    }
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
    /// The file selected in the file manager: see [`PatchTool::file_action`].
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
