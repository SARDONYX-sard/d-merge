//! The list of objects: the keyboard in it, what it shows, and its changes.

use std::rc::Rc;

use stdx::text::{cmp_ignore_case, contains_ignore_case};

use crate::patch::{FocusAt, ObjectList, PatchTool, tree, ui, workspace::PANDORA_DOC};

impl PatchTool {
    /// The keyboard goes to the object tree (Vim `Space e`, or the palette).
    pub(in crate::patch) fn enter_tree(&mut self, ui: &egui::Ui, editor: egui::Id) {
        self.tree_focus = true;
        self.tree_view = true;
        self.show_vars = false;
        self.filter.clear();
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// The keyboard goes back to the editor.
    pub(in crate::patch) const fn leave_tree(&mut self) {
        self.tree_focus = false;
        self.focus_editor = Some(FocusAt::Keep);
    }

    /// The flat list has the same keys as the tree, without the folding.
    pub(in crate::patch) fn list_keys(&mut self, ui: &egui::Ui, graph: &tree::Graph) {
        let rows = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let selected = self.selected.as_deref();
        let mut current = rows.iter().position(|&node| selected == Some(graph.id(node))).unwrap_or(0);
        let mut leave = false;
        for input in ui::tree::read_keys(ui, &mut self.list_leader) {
            leave |= input == tree::TreeInput::Leave;
            current = input.step(current, rows.len()).unwrap_or(current);
        }
        if let Some(&node) = rows.get(current) {
            self.selected = Some(graph.id(node).into());
        }
        if leave {
            self.leave_tree();
        }
    }

    /// Recomputes the flat list when the objects, the filter or "changed only" changed.
    ///
    /// New objects come first and show where they are linked: `+ #mymod$0  hkbFoo  ← #0106`.
    ///
    /// Without a filter the labels do not matter, so typing in the editor does not recompute it.
    pub(in crate::patch) fn refresh_list(&mut self, graph: &tree::Graph) {
        let generation = if self.filter.is_empty() { graph.tree_generation() } else { graph.generation() };
        let fresh = self.list.as_ref().is_some_and(|list| {
            list.generation == generation && list.filter == self.filter && list.changed_only == self.changed_only
        });
        if fresh {
            return;
        }
        let filter = self.filter.as_str();
        let rows = graph
            .nodes()
            .filter(|&node| !self.changed_only || graph.is_changed(node))
            .filter(|&node| contains_ignore_case(graph.label(node), filter))
            .collect();
        // Anim data is grouped by actor: see `ui::anim_tree`.
        let mut rows: Vec<tree::Node> = rows;
        if self.workspace.behaviors[self.active].is_anim_data() {
            rows.sort_by(|&a, &b| cmp_ignore_case(graph.id(a), graph.id(b)));
        }
        self.list = Some(ObjectList { generation, filter: self.filter.clone(), changed_only: self.changed_only, rows });
    }

    /// Deletes a new object, or drops the edits of a template object.
    pub(in crate::patch) fn revert_or_delete(&mut self, id: &str) {
        let behavior = &mut self.workspace.behaviors[self.active];
        behavior.edits.remove(id);
        let is_pandora_doc = id == PANDORA_DOC && behavior.pandora_doc.take().is_some();
        if behavior.added.remove(id).is_some() || is_pandora_doc {
            let neighbor = self.tabs.close(self.active, id);
            if self.selected.as_deref() == Some(id) {
                self.selected = neighbor.map(Rc::from);
            }
        }
        self.editor_for = None;
        self.refresh_work_check(id);
        self.object_changed(id);
    }

    pub(in crate::patch) fn new_object(&mut self, from: Option<&str>) {
        let code = self.mod_code.trim().to_string();
        let id = self.workspace.behaviors[self.active].add_object(&code, from);
        self.object_changed(&id);
        self.selected = Some(id.into());
    }
}
