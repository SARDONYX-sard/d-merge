//! The side bar below the behaviors: the objects of the active one.
//!
//! ```text
//! [Objects] [Events / Variables] [Search]  [+] [⧉] [↶]    <- new, duplicate, revert or delete
//! [filter......................]     [Δ] [≡]        <- only changed, tree or flat list
//! (the tree or the list)
//! ```
//! A window has the class list: nothing moves in the side bar when it is opened.

use egui_widgets::shadcn::{Button, Icon, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;
use stdx::text::contains_ignore_case;

use super::{
    anim_data::{DuplicateForm, NewClipsForm},
    anim_tree::{self, AnimFile, AnimRow, FileKind},
    behaviors::quick_pick_window,
    work_check::{CheckScope, check_items},
};
use crate::patch::{
    PatchTool,
    anim::data,
    tree::{Graph, Node, TreeState},
    workspace::PANDORA_DOC,
};

impl PatchTool {
    pub(in crate::patch) fn object_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let asdsf = self.workspace.behaviors[self.active].is_anim_data();
        // `Ctrl+Shift+F`, as in VS Code.
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::F)) {
            self.show_search = true;
            self.focus_search = true;
        }
        ui.horizontal(|ui| {
            self.list_tabs(ui, i18n, asdsf);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if (!self.show_vars || asdsf) && !self.show_search {
                    self.object_buttons(ui, i18n, asdsf);
                }
            });
        });
        if self.show_search {
            self.search_panel(ui, i18n);
            return;
        }
        ui.horizontal(|ui| {
            // Right to left: the toggles take their width first, the filter gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.show_vars || asdsf {
                    self.list_toggles(ui, i18n, asdsf);
                }
                ui.add(
                    TextEdit::singleline(&mut self.filter)
                        .hint_text(i18n.t(I18nKey::NemesisFilterHint))
                        .desired_width(ui.available_width()),
                );
            });
        });
        if self.show_vars && !asdsf {
            self.vars_panel(ui, i18n);
            return;
        }

        // Clicking any text field takes the keyboard out of the tree.
        if ui.memory(|m| m.focused().is_some()) {
            self.tree_focus = false;
        }
        let names = self.names();
        // A tree from the root container, unless filtering: a filter wants the flat list.
        if asdsf {
            self.anim_tree_panel(ui, i18n, &names.graph);
        } else if self.tree_view && self.filter.is_empty() {
            self.tree_panel(ui, i18n, &names.graph);
        } else {
            self.flat_list(ui, i18n, &names.graph);
        }
    }

    /// `[Objects] [Events / Variables] [Search]`. Anim data has no events or variables.
    fn list_tabs(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, asdsf: bool) {
        let objects = !self.show_vars && !self.show_search;
        if ui.add(Button::selectable(objects, i18n.t(I18nKey::NemesisObjectsTab)).small()).clicked() {
            self.show_vars = false;
            self.show_search = false;
        }
        if !asdsf
            && ui
                .add(Button::selectable(self.show_vars && !self.show_search, i18n.t(I18nKey::NemesisVarsTab)).small())
                .clicked()
        {
            self.show_vars = true;
            self.show_search = false;
        }
        if ui.add(Button::selectable(self.show_search, i18n.t(I18nKey::NemesisSearchTab)).small()).clicked() {
            self.show_search = true;
            self.focus_search = true;
        }
    }

    /// `[Δ] [≡]`: only what changed, a tree or a flat list.
    fn list_toggles(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, asdsf: bool) {
        if asdsf {
            let collapse = Button::new("").icon(Icon::ChevronRight).small();
            if ui.add(collapse).on_hover_text(i18n.t(I18nKey::CollapseAllButton)).clicked() {
                self.anim_folders_open.clear();
            }
            let expand = Button::new("").icon(Icon::ChevronDown).small();
            self.anim_expand_all |= ui.add(expand).on_hover_text(i18n.t(I18nKey::ExpandAllButton)).clicked();
        }
        if !asdsf {
            let tree = Button::selectable(self.tree_view, "").icon(Icon::ListTree).small();
            if ui.add(tree).on_hover_text(i18n.t(I18nKey::NemesisTreeViewHover)).clicked() {
                self.tree_view = !self.tree_view;
            }
        }
        let changed = Button::selectable(self.changed_only, "").icon(Icon::Diff).small();
        if ui.add(changed).on_hover_text(i18n.t(I18nKey::NemesisChangedOnly)).clicked() {
            self.changed_only = !self.changed_only;
        }
    }

    /// `[+] [⧉] [↶]`, drawn right to left. A new object is deleted, a template object goes back to
    /// the template. Anim sets can only be edited, not added; clip data adds clips ([`NewClipsForm`]).
    fn object_buttons(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, asdsf: bool) {
        if !self.workspace.behaviors[self.active].is_anim_data()
            && ui
                .add(Button::new("").icon(Icon::FilePlus).small())
                .on_hover_text(i18n.t(I18nKey::NemesisPandoraDocHover))
                .clicked()
        {
            self.open_pandora_doc();
        }
        let code_ok = is_existing_mod_code(self.mod_code.trim()) && !asdsf;
        let selected = self.selected.clone();
        let is_new = selected
            .as_ref()
            .is_some_and(|id| **id == *PANDORA_DOC || self.workspace.behaviors[self.active].added.contains_key(&**id));
        let is_pandora_doc = selected.as_deref() == Some(PANDORA_DOC);
        let (icon, hover) = if is_new {
            (Icon::Trash, I18nKey::NemesisDeleteObjectHover)
        } else {
            (Icon::Undo2, I18nKey::NemesisRevertHover)
        };
        if ui
            .add_enabled(selected.is_some(), Button::new("").icon(icon).small().destructive())
            .on_hover_text(i18n.t(hover))
            .clicked()
            && let Some(id) = &selected
        {
            self.revert_or_delete(id);
        }
        if ui
            .add_enabled(code_ok && selected.is_some() && !is_pandora_doc, Button::new("").icon(Icon::Copy).small())
            .on_hover_text(i18n.t(I18nKey::NemesisDuplicate))
            .clicked()
        {
            self.new_object(selected.as_deref());
        }
        let adsf = self.workspace.behaviors[self.active].adsf;
        let hover = if adsf { I18nKey::NemesisNewClipsHover } else { I18nKey::NemesisNewObject };
        if ui
            .add_enabled(
                code_ok || (adsf && is_existing_mod_code(self.mod_code.trim())),
                Button::new("").icon(Icon::Plus).small(),
            )
            .on_hover_text(i18n.t(hover))
            .on_disabled_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid))
            .clicked()
        {
            if adsf {
                let projects = self.anim_projects();
                self.new_clips = Some(NewClipsForm::new(selected.as_deref(), projects));
            } else {
                self.class_picker = true;
                self.focus_quick_pick = true;
            }
        }
    }

    /// The tree, and its keys while it has the keyboard.
    fn tree_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph) {
        let path = &self.workspace.behaviors[self.active].meshes_path;
        if !self.trees.contains_key(path) {
            self.trees.insert(path.clone(), TreeState::default());
        }
        let Some(tree) = self.trees.get_mut(path) else { return };
        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let inputs = super::tree::read_keys(ui, &mut tree.leader);
            if tree.apply_inputs(&inputs, graph, &mut self.selected, self.changed_only) {
                self.leave_tree();
                return;
            }
        }
        let flags = (self.changed_only, self.tree_focus);
        if let Some(id) = super::tree::tree_ui(ui, graph, &mut self.selected, tree, flags, i18n) {
            self.keep_open(id);
        }
    }

    /// The anim data files after the filter: the rows of [`anim_tree::rows`].
    fn anim_tree_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph) {
        self.refresh_list(graph);
        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            self.list_keys(ui, graph);
        }
        let is_adsf = self.workspace.behaviors[self.active].adsf;
        // Only when the selection changes: after that, what is open is the user's.
        if self.anim_revealed != self.selected {
            self.anim_revealed.clone_from(&self.selected);
            if let Some(id) = &self.selected {
                anim_tree::reveal(&mut self.anim_folders_open, is_adsf, id);
            }
        }
        let nodes = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let files: Vec<AnimFile<'_>> =
            nodes.iter().map(|&node| AnimFile { node, id: graph.id(node), changed: graph.is_changed(node) }).collect();
        if std::mem::take(&mut self.anim_expand_all) {
            anim_tree::expand_all(&mut self.anim_folders_open, &files);
        }
        let rows = anim_tree::rows(&files, &self.anim_folders_open, is_adsf, !self.filter.is_empty());

        let (mut toggled, mut menu_action) = (None, None);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_height, rows.len(), |ui, range| {
            for row in &rows[range] {
                match row {
                    AnimRow::Folder { name, files, changed, open } => {
                        let response = anim_group_row(ui, name, (*files, *changed, *open), 0);
                        if response.clicked() {
                            toggled = Some((*name).to_owned());
                        }
                        response.context_menu(|ui| {
                            if ui.button(i18n.t(I18nKey::NemesisExpandAllBelow)).clicked() {
                                anim_tree::expand(&mut self.anim_folders_open, name);
                                ui.close();
                            }
                            if ui.button(i18n.t(I18nKey::NemesisCollapseAllBelow)).clicked() {
                                anim_tree::collapse(&mut self.anim_folders_open, name);
                                ui.close();
                            }
                            ui.separator();
                            let projects = self.anim_projects();
                            if let Some(pick) = duplicate_items(ui, i18n, &projects, name) {
                                let files = data::changed_files(&self.workspace.behaviors[self.active], name);
                                self.ask_duplicate(pick, files, &projects, name);
                            }
                            if let Some(is_row) = check_items(ui, i18n) {
                                let folder = is_row.then(|| CheckScope::Folder((*name).to_owned()));
                                self.start_work_check(folder.unwrap_or(CheckScope::All));
                            }
                        });
                    }
                    AnimRow::Group { kind, key, files, changed, open } => {
                        let label = i18n.t(anim_kind_label(*kind));
                        let response = anim_group_row(ui, label, (*files, *changed, *open), 1);
                        if response.clicked() {
                            toggled = Some(key.clone());
                        }
                        // `DefaultMale~1/Clip`
                        let folder = key.rsplit_once('/').map_or(key.as_str(), |(folder, _)| folder);
                        response.context_menu(|ui| {
                            if let Some(is_row) = check_items(ui, i18n) {
                                let of_kind = CheckScope::Kind { folder: folder.to_owned(), kind: *kind };
                                self.start_work_check(if is_row { of_kind } else { CheckScope::All });
                            }
                        });
                    }
                    AnimRow::File { node, name } => {
                        menu_action = self.anim_file_row(ui, i18n, graph, *node, name).or(menu_action);
                    }
                }
            }
        });
        if let Some(key) = toggled {
            anim_tree::toggle(&mut self.anim_folders_open, &key);
        }
        self.apply_row_action(ui, graph, menu_action);
        self.duplicate_anim_files();
    }

    /// What a row's menu asked for: at once, or after asking where. `current`: the project they are in.
    fn ask_duplicate(&mut self, pick: DuplicatePick, files: Vec<String>, projects: &[String], current: &str) {
        match pick {
            DuplicatePick::To(to) => self.pending_duplicate = Some((files, to)),
            DuplicatePick::Choose => self.duplicate_form = Some(DuplicateForm::new(files, projects.to_vec(), current)),
        }
    }

    /// The files go to the other project, which opens to show them.
    fn duplicate_anim_files(&mut self) {
        let Some((files, to)) = self.pending_duplicate.take() else { return };
        let made = data::duplicate_to(&mut self.workspace.behaviors[self.active], &files, &to);
        for id in &made {
            self.object_changed(id);
        }
        self.anim_folders_open.insert(to);
    }

    /// `    ● Idle~mymod$0.txt`
    fn anim_file_row(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        graph: &Graph,
        node: Node,
        name: &str,
    ) -> Option<(Node, RowAction)> {
        let mut action = None;
        let is_selected = self.selected.as_deref() == Some(graph.id(node));
        let marker = if graph.is_changed(node) { "● " } else { "" };
        let text = egui::RichText::new(format!("    {marker}{name}")).monospace();
        let response = ui.add(egui::Button::selectable(is_selected, text));
        if response.clicked() {
            self.selected = Some(graph.id(node).into());
        }
        if response.double_clicked() {
            action = Some((node, RowAction::KeepOpen));
        }
        response.context_menu(|ui| {
            if let Some(picked) = row_menu(ui, i18n, graph.is_new(node), graph.is_changed(node)) {
                action = Some((node, picked));
            }
            let id = graph.id(node);
            let folder = id.split_once('/').map_or(id, |(folder, _)| folder);
            let projects = self.anim_projects();
            if let Some(pick) = duplicate_items(ui, i18n, &projects, folder) {
                let files = data::with_motion(&self.workspace.behaviors[self.active], id);
                self.ask_duplicate(pick, files, &projects, folder);
            }
        });
        action
    }

    /// The flat list after the filter (only the rows on screen are laid out).
    fn flat_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph) {
        self.refresh_list(graph);
        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            self.list_keys(ui, graph);
        }
        let rows = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let mut menu_action = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_height, rows.len(), |ui, range| {
            for &node in &rows[range] {
                let id = graph.id(node);
                let text = super::tree::object_label(ui, graph, node, false);
                let selected = self.selected.as_deref() == Some(id);
                let response = ui.add(egui::Button::selectable(selected, text));
                if response.clicked() {
                    self.selected = Some(id.into());
                }
                if response.double_clicked() {
                    menu_action = Some((node, RowAction::KeepOpen));
                }
                response.context_menu(|ui| {
                    if let Some(action) = row_menu(ui, i18n, graph.is_new(node), graph.is_changed(node)) {
                        menu_action = Some((node, action));
                    }
                });
            }
        });
        self.apply_row_action(ui, graph, menu_action);
    }

    fn apply_row_action(&mut self, ui: &egui::Ui, graph: &Graph, action: Option<(Node, RowAction)>) {
        let Some((node, action)) = action else { return };
        let id = graph.id(node);
        match action {
            RowAction::KeepOpen => self.keep_open(id),
            RowAction::Duplicate => self.new_object(Some(id)),
            RowAction::CopyId => ui.ctx().copy_text(id.to_owned()),
            RowAction::RevertOrDelete => self.revert_or_delete(id),
        }
    }

    /// The window of [`Self::class_picker_ui`].
    pub(in crate::patch) fn class_picker_window(&mut self, ctx: &egui::Context, i18n: &I18nMap) {
        if !self.class_picker {
            return;
        }
        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        quick_pick_window(ctx, i18n.t(I18nKey::NemesisNewObject))
            .open(&mut open)
            .show(ctx, |ui| self.class_picker_ui(ui, i18n));
        self.class_picker &= open; // a picked class has turned it off already
    }

    /// `[class filter]` + the class table's classes (the template's own first); a click adds a
    /// new object of that class with every field at its default.
    fn class_picker_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let field = ui.add(
            TextEdit::singleline(&mut self.class_filter)
                .hint_text(i18n.t(I18nKey::NemesisClassFilterHint))
                .desired_width(ui.available_width()),
        );
        if std::mem::take(&mut self.focus_quick_pick) {
            field.request_focus();
        }
        let behavior = &self.workspace.behaviors[self.active];
        let filter = self.class_filter.as_str();
        let used = |class: &str| behavior.object_of_class(class).is_some();
        let mut classes: Vec<&str> =
            nemesis_diff::schema::class_names().into_iter().filter(|c| contains_ignore_case(c, filter)).collect();
        classes.sort_by_key(|class| !used(class)); // stable: alphabetical within each group

        let mut picked = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::vertical().id_salt("nemesis_class_picker").max_height(420.0).show_rows(
            ui,
            row_height,
            classes.len(),
            |ui, range| {
                for class in &classes[range] {
                    let text = egui::RichText::new(*class).monospace();
                    let text = if used(class) { text } else { text.weak() };
                    if ui.add(egui::Button::selectable(false, text)).clicked() {
                        picked = Some((*class).to_string());
                    }
                }
            },
        );
        if let Some(class) = picked {
            let code = self.mod_code.trim().to_string();
            if let Some(id) = self.workspace.behaviors[self.active].add_class(&code, &class) {
                self.object_changed(&id);
                self.selected = Some(id.into());
                self.class_picker = false;
            }
        }
    }
}

/// What a row's menu asked to do with its files.
#[derive(Debug)]
enum DuplicatePick {
    To(String),
    /// Ask which project, see [`DuplicateForm`].
    Choose,
}

/// `Duplicate to DefaultFemale~1`, the project that usually has the same patches, if there is one, and
/// `Duplicate to...`, which asks. `current`: the project it is in.
fn duplicate_items(ui: &mut egui::Ui, i18n: &I18nMap, projects: &[String], current: &str) -> Option<DuplicatePick> {
    let mut picked = None;
    let hover = i18n.t(I18nKey::NemesisDuplicateToHover);
    let partner = data::partner_of(current);
    if partner != current && projects.contains(&partner) {
        let label = format!("{} {partner}", i18n.t(I18nKey::NemesisDuplicateTo));
        if ui.button(label).on_hover_text(hover).clicked() {
            picked = Some(DuplicatePick::To(partner));
            ui.close();
        }
    }
    if ui.button(i18n.t(I18nKey::NemesisDuplicateToOther)).on_hover_text(hover).clicked() {
        picked = Some(DuplicatePick::Choose);
        ui.close();
    }
    picked
}

/// A folder or a group of the anim data list: `▼ Clips (3) ●`. `(files, changed, open)`.
fn anim_group_row(
    ui: &mut egui::Ui,
    label: &str,
    (files, changed, open): (usize, bool, bool),
    depth: usize,
) -> egui::Response {
    let glyph = if open { "▼" } else { "▶" };
    let marker = if changed { " ●" } else { "" };
    let indent = "  ".repeat(depth);
    ui.add(egui::Button::selectable(
        false,
        egui::RichText::new(format!("{indent}{glyph} {label} ({files}){marker}")).monospace(),
    ))
}

const fn anim_kind_label(kind: FileKind) -> I18nKey {
    match kind {
        FileKind::Header => I18nKey::NemesisAnimKindHeader,
        FileKind::Clip => I18nKey::NemesisAnimKindClip,
        FileKind::Motion => I18nKey::NemesisAnimKindMotion,
        FileKind::Set => I18nKey::NemesisAnimKindSet,
        FileKind::NewSet => I18nKey::NemesisAnimKindNewSet,
    }
}

/// What a row's right-click menu asked for.
#[derive(Debug, Clone, Copy)]
enum RowAction {
    KeepOpen,
    Duplicate,
    CopyId,
    RevertOrDelete,
}

/// `Keep Open / Duplicate / Copy id / Revert (or Delete for a new object)`.
fn row_menu(ui: &mut egui::Ui, i18n: &I18nMap, is_new: bool, changed: bool) -> Option<RowAction> {
    let keep_open = ui.button(i18n.t(I18nKey::NemesisKeepOpen)).clicked();
    let duplicate = ui.button(i18n.t(I18nKey::NemesisDuplicate)).clicked();
    let copy = ui.button(i18n.t(I18nKey::NemesisCopyId)).clicked();
    let revert = changed && {
        ui.separator();
        let label = if is_new { I18nKey::NemesisDeleteObject } else { I18nKey::RevertButton };
        ui.add(Button::new(i18n.t(label)).destructive()).clicked()
    };
    [
        (keep_open, RowAction::KeepOpen),
        (duplicate, RowAction::Duplicate),
        (copy, RowAction::CopyId),
        (revert, RowAction::RevertOrDelete),
    ]
    .into_iter()
    .find_map(|(clicked, action)| clicked.then_some(action))
}
