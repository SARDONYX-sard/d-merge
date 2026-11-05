//! The left panel: the object list (a tree or a flat list), its buttons and the class picker.

use egui_widgets::shadcn::{Button, Checkbox, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_valid_mod_code;

use crate::patch::{PatchTool, tree::Graph};

impl PatchTool {
    pub(in crate::patch) fn object_list(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let asdsf = self.workspace.behaviors[self.active].asdsf;
        if !asdsf {
            self.list_tabs(ui, i18n);
        }
        ui.add(
            TextEdit::singleline(&mut self.filter)
                .hint_text(i18n.t(I18nKey::NemesisFilterHint))
                .desired_width(ui.available_width()),
        );
        if self.show_vars && !asdsf {
            self.vars_panel(ui, i18n);
            return;
        }
        ui.horizontal(|ui| {
            ui.add(Checkbox::new(&mut self.changed_only, i18n.t(I18nKey::NemesisChangedOnly)));
            if !asdsf {
                ui.add(Checkbox::new(&mut self.tree_view, i18n.t(I18nKey::NemesisTreeView)))
                    .on_hover_text(i18n.t(I18nKey::NemesisTreeViewHover));
            }
        });
        self.object_buttons(ui, i18n, asdsf);
        if self.class_picker {
            self.class_picker_ui(ui, i18n);
        }
        ui.separator();

        // Clicking any text field takes the keyboard out of the tree.
        if ui.memory(|m| m.focused().is_some()) {
            self.tree_focus = false;
        }
        let names = self.names();
        // A tree from the root container, unless filtering: a filter wants the flat list.
        if self.tree_view && self.filter.is_empty() && !asdsf {
            self.tree_panel(ui, i18n, &names.graph);
        } else {
            self.flat_list(ui, i18n, &names.graph);
        }
    }

    /// `[Objects] [Events / Variables]`
    fn list_tabs(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.horizontal(|ui| {
            for (vars, label) in [(false, I18nKey::NemesisObjectsTab), (true, I18nKey::NemesisVarsTab)] {
                if ui.add(Button::selectable(self.show_vars == vars, i18n.t(label))).clicked() {
                    self.show_vars = vars;
                }
            }
        });
    }

    /// `[New] [Duplicate] [Revert]`. A new object is deleted, a template object goes back to the
    /// template. Anim sets (`asdsf`) can only be edited, not added.
    fn object_buttons(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, asdsf: bool) {
        ui.horizontal(|ui| {
            let code_ok = is_valid_mod_code(self.mod_code.trim()) && !asdsf;
            if ui
                .add_enabled(code_ok, Button::new(i18n.t(I18nKey::NemesisNewObject)).small())
                .on_disabled_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid))
                .clicked()
            {
                self.class_picker = !self.class_picker; // pick the class below
            }
            let selected = self.selected.clone();
            if ui
                .add_enabled(code_ok && selected.is_some(), Button::new(i18n.t(I18nKey::NemesisDuplicate)).small())
                .clicked()
            {
                self.new_object(selected.as_deref());
            }
            let is_new =
                selected.as_ref().is_some_and(|id| self.workspace.behaviors[self.active].added.contains_key(id));
            let (label, hover) = if is_new {
                (I18nKey::NemesisDeleteObject, I18nKey::NemesisDeleteObjectHover)
            } else {
                (I18nKey::NemesisRevert, I18nKey::NemesisRevertHover)
            };
            if ui
                .add_enabled(selected.is_some(), Button::new(i18n.t(label)).small().destructive())
                .on_hover_text(i18n.t(hover))
                .clicked()
                && let Some(id) = selected
            {
                self.revert_or_delete(&id);
            }
        });
    }

    /// The tree, and its keys while it has the keyboard.
    fn tree_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, graph: &Graph) {
        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let inputs = super::tree::read_keys(ui, &mut self.tree.leader);
            if self.tree.apply_inputs(&inputs, graph, &mut self.selected, self.changed_only) {
                self.leave_tree();
            }
        }
        let flags = (self.changed_only, self.tree_focus);
        super::tree::tree_ui(ui, graph, &mut self.selected, &mut self.tree, flags, i18n);
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
                    id.clone_into(self.selected.get_or_insert_default());
                }
                response.context_menu(|ui| {
                    if let Some(action) = row_menu(ui, i18n, graph.is_new(node), graph.is_changed(node)) {
                        menu_action = Some((node, action));
                    }
                });
            }
        });
        if let Some((node, action)) = menu_action {
            let id = graph.id(node);
            match action {
                RowAction::Duplicate => self.new_object(Some(id)),
                RowAction::CopyId => ui.ctx().copy_text(id.to_owned()),
                RowAction::RevertOrDelete => self.revert_or_delete(id),
            }
        }
    }

    /// `[class filter]` + the class table's classes (the template's own first); a click adds a
    /// new object of that class with every field at its default.
    fn class_picker_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        ui.add(
            TextEdit::singleline(&mut self.class_filter)
                .hint_text(i18n.t(I18nKey::NemesisClassFilterHint))
                .desired_width(ui.available_width()),
        );
        let behavior = &self.workspace.behaviors[self.active];
        let filter = self.class_filter.to_lowercase();
        let used = |class: &str| behavior.object_of_class(class).is_some();
        let mut classes: Vec<&str> =
            nemesis_diff::schema::class_names().into_iter().filter(|c| c.to_lowercase().contains(&filter)).collect();
        classes.sort_by_key(|class| !used(class)); // stable: alphabetical within each group

        let mut picked = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::vertical().id_salt("nemesis_class_picker").max_height(200.0).show_rows(
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
                self.selected = Some(id);
                self.class_picker = false;
            }
        }
    }
}

/// What a row's right-click menu asked for.
#[derive(Debug, Clone, Copy)]
enum RowAction {
    Duplicate,
    CopyId,
    RevertOrDelete,
}

/// `Duplicate / Copy id / Revert (or Delete for a new object)`.
fn row_menu(ui: &mut egui::Ui, i18n: &I18nMap, is_new: bool, changed: bool) -> Option<RowAction> {
    let duplicate = ui.button(i18n.t(I18nKey::NemesisDuplicate)).clicked();
    let copy = ui.button(i18n.t(I18nKey::NemesisCopyId)).clicked();
    let revert = changed && {
        ui.separator();
        let label = if is_new { I18nKey::NemesisDeleteObject } else { I18nKey::NemesisRevert };
        ui.add(Button::new(i18n.t(label)).destructive()).clicked()
    };
    [(duplicate, RowAction::Duplicate), (copy, RowAction::CopyId), (revert, RowAction::RevertOrDelete)]
        .into_iter()
        .find_map(|(clicked, action)| clicked.then_some(action))
}
