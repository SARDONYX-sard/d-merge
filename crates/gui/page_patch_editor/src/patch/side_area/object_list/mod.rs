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

use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;

use self::{
    anim_list::{DuplicatePick, Files},
    explorer::ListOutput,
};
use super::{SideArea, SideAsk};
use crate::patch::{
    anim::data,
    tool::{file::FileAction, keys::Keys, selection::Selection, work_state::Work},
    tree::Graph,
    ui::{
        anim_data::{anim_projects, duplicate_files},
        windows::WindowAsk,
    },
    workspace::PANDORA_DOC,
};

impl SideArea {
    /// See the module docs. Returns what it asked of the rest.
    pub(super) fn object_list(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (work, sel, keys): (&mut Work, &mut Selection, &mut Keys),
    ) -> Vec<SideAsk> {
        let mut asks = Vec::new();
        let anim = work.workspace.behaviors[sel.active].is_anim_data();
        let mut open = !self.bar.objects_closed;
        let asked = {
            let title = self.title(work, sel, anim);
            let mut asked = None;
            egui_widgets::section_header(ui, &mut open, i18n.t(I18nKey::NemesisObjectsTab), |ui| {
                asked = title::buttons(ui, i18n, title);
            });
            asked
        };
        if let Some(asked) = asked {
            asks.extend(self.apply_title((work, sel), asked));
        }
        self.bar.objects_closed = !open;
        if self.bar.objects_closed {
            return asks;
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
        self.filter_keys(keys, anim);

        let names = work.names(sel.active);
        let graph = &names.graph;
        let tree_keys =
            if self.explorer.tree_focus { keys.take_mapped(keybinding::host::tree_key) } else { Vec::new() };

        // A tree from the root container, unless filtering: a filter wants the flat list.
        let output = if anim {
            let projects = anim_projects(work, sel.active);
            let is_adsf = work.workspace.behaviors[sel.active].adsf;
            self.explorer.anim(ui, i18n, (graph, is_adsf, &projects), &tree_keys, &mut sel.selected)
        } else if self.explorer.tree_view && self.explorer.filter.is_empty() && !self.explorer.unlinked_only {
            let path = &work.workspace.behaviors[sel.active].meshes_path;
            self.explorer.tree(ui, i18n, (graph, path), &tree_keys, &mut sel.selected)
        } else {
            self.explorer.flat(ui, i18n, graph, &tree_keys, &mut sel.selected)
        };
        asks.extend(self.apply_list_output(ui, (work, sel), graph, output));
        asks
    }

    /// The title's buttons, of what is shown.
    fn title<'a>(&'a mut self, work: &'a Work, sel: &'a Selection, anim: bool) -> title::Title<'a> {
        let behavior = &work.workspace.behaviors[sel.active];
        let selected = sel.selected.as_ref();
        title::Title {
            anim,
            adsf: behavior.adsf,
            tree_view: self.explorer.tree_view,
            code_ok: is_existing_mod_code(work.place.mod_code.trim()),
            selected,
            selected_is_new: selected.is_some_and(|id| **id == *PANDORA_DOC || behavior.added.contains_key(&**id)),
            selected_is_pandora_doc: selected.is_some_and(|id| **id == *PANDORA_DOC),
            changed_only: &mut self.explorer.changed_only,
            unlinked_only: &mut self.explorer.unlinked_only,
        }
    }

    /// `t` / `c` / `u` (key bindings): what the list shows, as the `[≡]` / `⋯` menu does.
    fn filter_keys(&mut self, keys: &mut Keys, anim: bool) {
        use keybinding::commands::{EXPLORER_TOGGLE_CHANGED_ONLY, EXPLORER_TOGGLE_UNLINKED_ONLY, EXPLORER_TOGGLE_VIEW};
        self.explorer.tree_view ^= keys.take(EXPLORER_TOGGLE_VIEW) && !anim;
        self.explorer.changed_only ^= keys.take(EXPLORER_TOGGLE_CHANGED_ONLY);
        self.explorer.unlinked_only ^= keys.take(EXPLORER_TOGGLE_UNLINKED_ONLY) && !anim;
    }

    /// What the list asked for: back to the editor, a row's menu, a duplicate.
    fn apply_list_output(
        &mut self,
        ui: &egui::Ui,
        (work, sel): (&mut Work, &mut Selection),
        graph: &Graph,
        output: ListOutput,
    ) -> Vec<SideAsk> {
        let mut asks = Vec::new();
        if output.leave {
            self.explorer.tree_focus = false;
            asks.push(SideAsk::FocusEditor);
        }
        if let Some((node, action)) = output.menu {
            asks.extend(row_action(ui, (work, sel), (graph.id(node), action)));
        }

        let Some((pick, files, from)) = output.duplicate else { return asks };
        let files = {
            let behavior = &work.workspace.behaviors[sel.active];
            match files {
                Files::ChangedOf(folder) => data::changed_files(behavior, &folder),
                Files::WithMotion(id) => data::with_motion(behavior, &id),
            }
        };
        match pick {
            DuplicatePick::To(to) => {
                duplicate_files(work, sel.active, (&files, &to));
                self.explorer.anim_tree.open.insert(to); // it opens to show them
            }
            DuplicatePick::Choose => {
                let projects = anim_projects(work, sel.active);
                asks.push(SideAsk::Window(WindowAsk::Duplicate(files, projects, from)));
            }
        }
        asks
    }

    /// What a button of the title asked for.
    fn apply_title(&mut self, (work, sel): (&mut Work, &mut Selection), asked: title::TitleAsk) -> Option<SideAsk> {
        use title::TitleAsk;
        match asked {
            TitleAsk::ToggleTreeView => self.explorer.tree_view = !self.explorer.tree_view,
            TitleAsk::ExpandAll => self.explorer.anim_tree.expand_all = true,
            TitleAsk::CollapseAll => self.explorer.anim_tree.open.clear(),

            TitleAsk::OpenPandoraDoc => return Some(SideAsk::OpenPandoraDoc),
            TitleAsk::Duplicate(id) => sel.selected = Some(work.new_object(sel.active, Some(&id)).into()),
            TitleAsk::RevertOrDelete(id) => return Some(SideAsk::RevertOrDelete(id.to_string())),

            TitleAsk::New if work.workspace.behaviors[sel.active].adsf => {
                let projects = anim_projects(work, sel.active);
                let selected = sel.selected.as_deref().map(str::to_owned);
                return Some(SideAsk::Window(WindowAsk::NewClips(selected, projects)));
            }
            TitleAsk::New => return Some(SideAsk::Window(WindowAsk::ClassPicker)),
        }
        None
    }
}

/// What a row's menu asked for, of object `id`.
fn row_action(
    ui: &egui::Ui,
    (work, sel): (&mut Work, &mut Selection),
    (id, action): (&str, RowAction),
) -> Option<SideAsk> {
    match action {
        RowAction::KeepOpen => return Some(SideAsk::KeepOpen(id.to_owned())),
        RowAction::NewObject => return Some(SideAsk::Window(WindowAsk::ClassPicker)),
        RowAction::Duplicate => sel.selected = Some(work.new_object(sel.active, Some(id)).into()),
        RowAction::Repeat => return Some(SideAsk::Window(WindowAsk::Repeat(sel.active, id.to_owned()))),
        RowAction::CopyId => ui.ctx().copy_text(id.to_owned()),
        RowAction::RevertOrDelete => return Some(SideAsk::RevertOrDelete(id.to_owned())),
        RowAction::CopyPath => return Some(SideAsk::File(FileAction::CopyPath, id.to_owned())),
        RowAction::Reveal => return Some(SideAsk::File(FileAction::Reveal, id.to_owned())),
    }
    None
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
    /// The path of the file the object is in: see [`crate::patch::tool::file::object_file`].
    CopyPath,
    /// The file selected in the file manager: see [`crate::patch::tool::file::file_action`].
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
