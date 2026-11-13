//! The OBJECTS section of the explorer, below the behaviors: the objects of the active one.
//!
//! ```text
//! ▼ OBJECTS          [+] [≡] [⋯]    <- new, tree or flat list, the rest (see `more_menu`)
//! [filter.....................]
//! (the tree or the list)
//! ```
//! The title's actions are the explorer's: what is done to one object is in its right-click menu.
//! A window has the class list: nothing moves in the side bar when it is opened.
mod title;

use egui_widgets::{
    notify::Notification,
    shadcn::{Button, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_existing_mod_code;
use stdx::text::contains_ignore_case;

use super::{
    anim_data::{DuplicateForm, NewClipsForm},
    anim_tree::{self, AnimFile, AnimRow, FileKind},
};
use crate::patch::{
    PatchTool,
    anim::data,
    tool::list::{fixed_row, offset_to_show, row_height, take_reveal},
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
                self.panels.class_picker = true;
                self.panels.focus_quick_pick = true;
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
        let nodes = self.explorer.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let files: Vec<AnimFile<'_>> =
            nodes.iter().map(|&node| AnimFile { node, id: graph.id(node), changed: graph.is_changed(node) }).collect();
        if std::mem::take(&mut self.anim_tree.expand_all) {
            anim_tree::expand_all(&mut self.anim_tree.open, &files);
        }
        let filtering = !self.explorer.filter.is_empty();
        let mut rows = anim_tree::rows(&files, &self.anim_tree.open, is_adsf, filtering);

        // The keys on the rows shown, as in the object tree; a file the cursor gets to opens.
        let mut cursor_moved = false;
        if self.explorer.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let inputs = keybinding::host::take_mapped(&mut self.key_commands, keybinding::host::tree_key);
            for input in inputs {
                let nav: Vec<_> = rows.iter().map(AnimRow::nav).collect();
                match egui_tree::navigate(input, &nav, self.anim_tree.cursor) {
                    egui_tree::Nav::Cursor(at) => {
                        self.anim_tree.cursor = at;
                        cursor_moved = true;
                    }
                    egui_tree::Nav::SetOpen(at, _) => {
                        anim_tree::toggle(&mut self.anim_tree.open, rows[at].open_key());
                        rows = anim_tree::rows(&files, &self.anim_tree.open, is_adsf, filtering);
                    }
                    egui_tree::Nav::Leave => self.leave_tree(),
                    egui_tree::Nav::None => {}
                }
            }
            if cursor_moved && let Some(AnimRow::File { node, .. }) = rows.get(self.anim_tree.cursor) {
                self.selected = Some(graph.id(*node).into());
            }
        } else if let Some(at) = self
            .selected
            .as_deref()
            .and_then(|id| graph.node_of(id))
            .and_then(|node| rows.iter().position(|row| matches!(row, AnimRow::File { node: n, .. } if *n == node)))
        {
            // The keyboard starts where the selection is.
            self.anim_tree.cursor = at;
        }
        self.anim_tree.cursor = self.anim_tree.cursor.min(rows.len().saturating_sub(1));
        let focused_row = self.explorer.tree_focus.then_some(self.anim_tree.cursor);

        let (mut toggled, mut menu_action) = (None, None);
        let row_height = row_height(ui);
        // Up and down only, as VS Code's: the rows are cut with `…`. (Sideways, its width changed with the
        // rows drawn, and the scroll bars came and went.)
        let mut scroll = egui::ScrollArea::vertical().auto_shrink(false);
        let selected = self.selected.as_deref().and_then(|id| graph.node_of(id));
        let at = selected
            .and_then(|node| rows.iter().position(|row| matches!(row, AnimRow::File { node: n, .. } if *n == node)));
        let sizes = (row_height, ui.spacing().item_spacing.y);
        let to = if cursor_moved { focused_row } else { at.filter(|_| reveal) };
        if let Some(offset) = to.and_then(|at| offset_to_show(at, sizes, rows.len(), self.explorer.list_scroll, 0)) {
            scroll = scroll.vertical_scroll_offset(offset);
        }
        let output = scroll.show_rows(ui, row_height, rows.len(), |ui, range| {
            let first = range.start;
            for (offset, row) in rows[range].iter().enumerate() {
                // The keyboard's row has a frame, as in the object tree.
                if focused_row == Some(first + offset) {
                    let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), row_height));
                    egui_tree::list::cursor_frame(ui, rect);
                }
                fixed_row(ui, row_height, |ui| match row {
                    AnimRow::Folder { name, files, changed, open } => {
                        let response = anim_group_row(ui, name, (*files, *changed, *open), 0);
                        if response.clicked() {
                            toggled = Some((*name).to_owned());
                        }
                        response.context_menu(|ui| {
                            if ui.button(i18n.t(I18nKey::NemesisExpandAllBelow)).clicked() {
                                anim_tree::expand(&mut self.anim_tree.open, name);
                                ui.close();
                            }
                            if ui.button(i18n.t(I18nKey::NemesisCollapseAllBelow)).clicked() {
                                anim_tree::collapse(&mut self.anim_tree.open, name);
                                ui.close();
                            }
                            ui.separator();
                            let projects = self.anim_projects();
                            if let Some(pick) = duplicate_items(ui, i18n, &projects, name) {
                                let files = data::changed_files(&self.workspace.behaviors[self.active], name);
                                self.ask_duplicate(pick, files, &projects, name);
                            }
                        });
                    }
                    AnimRow::Group { kind, key, files, changed, open } => {
                        let label = i18n.t(anim_kind_label(*kind));
                        let response = anim_group_row(ui, label, (*files, *changed, *open), 1);
                        if response.clicked() {
                            toggled = Some(key.clone());
                        }
                    }
                    AnimRow::File { node, name } => {
                        menu_action = self.anim_file_row(ui, i18n, graph, *node, name).or(menu_action);
                    }
                });
            }
        });
        self.explorer.list_scroll.note(&output);
        if let Some(key) = toggled {
            anim_tree::toggle(&mut self.anim_tree.open, &key);
        }
        self.apply_row_action(ui, graph, menu_action, notify);
        self.duplicate_anim_files();
    }

    /// What a row's menu asked for: at once, or after asking where. `current`: the project they are in.
    fn ask_duplicate(&mut self, pick: DuplicatePick, files: Vec<String>, projects: &[String], current: &str) {
        match pick {
            DuplicatePick::To(to) => self.panels.pending_duplicate = Some((files, to)),
            DuplicatePick::Choose => {
                self.panels.duplicate_form = Some(DuplicateForm::new(files, projects.to_vec(), current));
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
        let response = egui_widgets::fitted_button(ui, is_selected, &format!("    {marker}{name}"));
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
    fn flat_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph, notify: &mut Notification) {
        self.refresh_list(graph);
        if self.explorer.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            self.list_keys(graph);
        }
        let rows = self.explorer.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let mut menu_action = None;
        let row_height = row_height(ui);
        // Up and down only, as VS Code's: the rows are cut with `…`. (Sideways, its width changed with the
        // rows drawn, and the scroll bars came and went.)
        let mut scroll = egui::ScrollArea::vertical().auto_shrink(false);
        let forced = std::mem::take(&mut self.explorer.reveal);
        if take_reveal(self.selected.as_ref(), forced) {
            let selected = self.selected.as_deref().and_then(|id| graph.node_of(id));
            self.explorer.list_scroll_to = selected.and_then(|node| rows.iter().position(|&row| row == node));
        }
        let sizes = (row_height, ui.spacing().item_spacing.y);
        let to = self.explorer.list_scroll_to.take();
        if let Some(offset) = to.and_then(|at| offset_to_show(at, sizes, rows.len(), self.explorer.list_scroll, 0)) {
            scroll = scroll.vertical_scroll_offset(offset);
        }
        let output = scroll.show_rows(ui, row_height, rows.len(), |ui, range| {
            for &node in &rows[range] {
                let id = graph.id(node);
                let selected = self.selected.as_deref() == Some(id);
                let response =
                    fixed_row(ui, row_height, |ui| super::tree::object_button(ui, graph, node, (selected, false)));
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
        self.explorer.list_scroll.note(&output);
        self.apply_row_action(ui, graph, menu_action, notify);
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
                self.panels.class_picker = true;
                self.panels.focus_quick_pick = true;
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

    /// The window of [`Self::class_picker_ui`].
    pub(in crate::patch) fn class_picker_window(&mut self, ctx: &egui::Context, i18n: &I18nMap) {
        if !self.panels.class_picker {
            return;
        }
        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        egui_widgets::quick_pick_window(ctx, i18n.t(I18nKey::NemesisNewObject))
            .open(&mut open)
            .show(ctx, |ui| self.class_picker_ui(ui, i18n));
        self.panels.class_picker &= open; // a picked class has turned it off already
    }

    /// `[class filter]` + the class table's classes (the template's own first); a click adds a
    /// new object of that class with every field at its default.
    fn class_picker_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let field = ui.add(
            TextEdit::singleline(&mut self.panels.class_filter)
                .hint_text(i18n.t(I18nKey::NemesisClassFilterHint))
                .desired_width(ui.available_width()),
        );
        if std::mem::take(&mut self.panels.focus_quick_pick) {
            field.request_focus();
        }
        let behavior = &self.workspace.behaviors[self.active];
        let filter = self.panels.class_filter.as_str();
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
            let code = self.place.mod_code.trim().to_string();
            if let Some(id) = self.workspace.behaviors[self.active].add_class(&code, &class) {
                self.object_changed(&id);
                self.selected = Some(id.into());
                self.panels.class_picker = false;
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
    egui_widgets::fitted_button(ui, false, &format!("{indent}{glyph} {label} ({files}){marker}"))
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
