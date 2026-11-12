//! The list of objects: the keyboard in it, what it shows, and its changes.

use std::rc::Rc;

use stdx::text::{cmp_ignore_case, contains_ignore_case};

use crate::patch::{FocusAt, ObjectList, PatchTool, tree, ui, workspace::PANDORA_DOC};

/// Rows kept above a revealed row, so that it is not at the very top of the list.
const ROWS_ABOVE_REVEALED: usize = 3;

/// Should a list show the selection again? When it changed since `revealed`, or when `forced` (the
/// editor got the keyboard: like VS Code, the side bar follows the file being edited). Notes the
/// selection as revealed.
///
/// e.g. `revealed` = `#0106`, `selected` = `#0106`: only if `forced`. `selected` = `#0107`: yes.
pub(in crate::patch) fn take_reveal(revealed: &mut Option<Rc<str>>, selected: Option<&Rc<str>>, forced: bool) -> bool {
    let changed = revealed.as_ref() != selected;
    if changed {
        *revealed = selected.cloned();
    }
    (changed || forced) && selected.is_some()
}

/// The scroll offset showing `row` of a list of rows of `row_height` (spacing included), with a few
/// rows above it. e.g. row 10, 20 px: 140 px (row 7 at the top).
pub(in crate::patch) fn reveal_offset(row: usize, row_height: f32) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "far fewer rows than f32 can count")]
    let top = row.saturating_sub(ROWS_ABOVE_REVEALED) as f32;
    top * row_height
}

impl PatchTool {
    /// Notes whether the editor `has_focus` this frame. Getting it asks the side bar to show the
    /// object again (next frame: the side bar is drawn first).
    pub(in crate::patch) fn note_editor_focus(&mut self, ctx: &egui::Context, has_focus: bool) {
        if has_focus && !self.editor_had_focus {
            self.reveal_in_side_bar = true;
            ctx.request_repaint();
        }
        self.editor_had_focus = has_focus;
    }

    /// The keyboard goes to the object tree (Vim `Space e`, or the palette).
    pub(in crate::patch) fn enter_tree(&mut self, ui: &egui::Ui, editor: egui::Id) {
        self.tree_focus = true;
        self.tree_view = true;
        self.side_view = crate::patch::ui::side_bar::SideView::Explorer;
        self.side_bar_hidden = false;
        self.objects_closed = false;
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

    /// Recomputes the flat list when the objects, the filter, "changed only" or "unlinked only" changed.
    ///
    /// New objects come first and show where they are linked: `+ #mymod$0  hkbFoo  ← #0106`.
    ///
    /// Without a filter the labels do not matter, so typing in the editor does not recompute it.
    pub(in crate::patch) fn refresh_list(&mut self, graph: &tree::Graph) {
        let generation = if self.filter.is_empty() { graph.tree_generation() } else { graph.generation() };
        let fresh = self.list.as_ref().is_some_and(|list| {
            list.generation == generation
                && list.filter == self.filter
                && list.changed_only == self.changed_only
                && list.unlinked_only == self.unlinked_only
        });
        if fresh {
            return;
        }
        let filter = self.filter.as_str();
        let rows = graph
            .nodes()
            .filter(|&node| !self.changed_only || graph.is_changed(node))
            .filter(|&node| !self.unlinked_only || graph.is_unlinked(node))
            .filter(|&node| contains_ignore_case(graph.label(node), filter))
            .collect();
        // Anim data is grouped by actor: see `ui::anim_tree`.
        let mut rows: Vec<tree::Node> = rows;
        if self.workspace.behaviors[self.active].is_anim_data() {
            rows.sort_by(|&a, &b| cmp_ignore_case(graph.id(a), graph.id(b)));
        }
        self.list = Some(ObjectList {
            generation,
            filter: self.filter.clone(),
            changed_only: self.changed_only,
            unlinked_only: self.unlinked_only,
            rows,
        });
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
        self.refresh_problems(id);
        self.object_changed(id);
    }

    pub(in crate::patch) fn new_object(&mut self, from: Option<&str>) {
        let code = self.mod_code.trim().to_string();
        let id = self.workspace.behaviors[self.active].add_object(&code, from);
        self.object_changed(&id);
        self.selected = Some(id.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_shows_a_new_selection_or_when_the_editor_gets_the_keyboard() {
        let (a, b): (Rc<str>, Rc<str>) = ("#a".into(), "#b".into());
        let mut revealed = None;
        assert!(take_reveal(&mut revealed, Some(&a), false), "new");
        assert!(!take_reveal(&mut revealed, Some(&a), false), "shown already");
        assert!(take_reveal(&mut revealed, Some(&a), true), "the editor got the keyboard");
        assert!(take_reveal(&mut revealed, Some(&b), false), "another one");
        assert!(!take_reveal(&mut revealed, None, true), "nothing to show");
        assert_eq!(revealed, None);
    }

    #[test]
    fn a_revealed_row_has_rows_above_it() {
        assert_eq!(reveal_offset(10, 20.0), 140.0);
        assert_eq!(reveal_offset(1, 20.0), 0.0);
    }

    /// Only getting the keyboard asks: keeping it does not, or the side bar could not be scrolled.
    #[test]
    fn getting_into_the_editor_asks_the_side_bar_to_follow() {
        let ctx = egui::Context::default();
        let mut tool = PatchTool::default();
        tool.note_editor_focus(&ctx, true);
        assert!(std::mem::take(&mut tool.reveal_in_side_bar));
        tool.note_editor_focus(&ctx, true);
        assert!(!tool.reveal_in_side_bar);
        tool.note_editor_focus(&ctx, false);
        tool.note_editor_focus(&ctx, true);
        assert!(tool.reveal_in_side_bar);
    }
}
