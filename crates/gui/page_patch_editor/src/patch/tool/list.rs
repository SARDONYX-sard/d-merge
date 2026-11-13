//! The list of objects: the keyboard in it, what it shows, and its changes.

use std::rc::Rc;

use stdx::text::{cmp_ignore_case, contains_ignore_case};

use crate::patch::{FocusAt, ObjectList, PatchTool, tree, ui, workspace::PANDORA_DOC};

/// The state of the objects in the explorer.
///
/// ```text
/// [filter.......]  ⋯ changed only / unlinked only    [≡] tree_view
/// tree (trees, by behavior) or flat list (list)       tree_focus: the keyboard is in it
/// ```
#[derive(Debug, Default)]
pub(in crate::patch) struct Explorer {
    pub(in crate::patch) filter: String,
    /// Only the objects changed, and the way to them in the tree.
    pub(in crate::patch) changed_only: bool,
    /// Only the new objects nothing links to. See [`crate::patch::tree::Graph::unlinked`].
    pub(in crate::patch) unlinked_only: bool,
    /// A tree from the root container, else a flat list.
    pub(in crate::patch) tree_view: bool,
    /// The tree of each behavior, by template path.
    pub(in crate::patch) trees: fast_hash::HashMap<String, crate::patch::tree::TreeState>,
    /// The flat list, as it was computed. See [`PatchTool::refresh_list`].
    pub(in crate::patch) list: Option<crate::patch::ObjectList>,

    /// `Space` was pressed with the keyboard in the flat list. See [`crate::patch::tree::TreeState::leader`].
    pub(in crate::patch) list_leader: bool,
    /// Where the flat list (or the animation data's tree) was scrolled. See [`offset_to_show`].
    pub(in crate::patch) list_scroll: ListScroll,
    /// The row of the flat list to scroll to: the keys moved to it, or the editor got the keyboard.
    pub(in crate::patch) list_scroll_to: Option<usize>,
    /// The keyboard is in the tree or the list (Vim `Space e`).
    pub(in crate::patch) tree_focus: bool,
    /// Show the selected object again, even if it was shown already: the editor got the keyboard.
    pub(in crate::patch) reveal: bool,
}

/// Should a list scroll to the selection? Only when `forced`: the editor got the keyboard, as VS Code's
/// explorer follows the file being edited. A selection alone moves nothing, or a list being used would.
pub(in crate::patch) const fn take_reveal(selected: Option<&Rc<str>>, forced: bool) -> bool {
    forced && selected.is_some()
}
/// Where a list of rows (`ScrollArea::show_rows`) was scrolled when last drawn.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(in crate::patch) struct ListScroll {
    /// The vertical offset, e.g. `120.0`.
    pub(in crate::patch) offset: f32,
    /// The height of what is seen of the list.
    pub(in crate::patch) view: f32,
}

impl ListScroll {
    /// Notes where the list is now.
    #[inline]
    pub(in crate::patch) fn note<R>(&mut self, output: &egui::scroll_area::ScrollAreaOutput<R>) {
        (self.offset, self.view) = (output.state.clamped_offset().y, output.inner_rect.height());
    }
}

/// The height of a row of the tree and of the lists: each row is drawn exactly that high
/// ([`fixed_row`]), or `show_rows` would place them, and scroll to them, wrongly.
pub(in crate::patch) fn row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Monospace) + 4.0)
}

/// Draws a row of `height` exactly, whatever is in it: the rows of a `show_rows` list must all be
/// of the height it was given. What is wider than the view still widens the list.
pub(in crate::patch) fn fixed_row<R>(ui: &mut egui::Ui, height: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let top_left = ui.cursor().min;
    let max_rect = egui::Rect::from_min_size(top_left, egui::vec2(ui.available_width().max(1.0), height));
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    let mut row = ui.new_child(egui::UiBuilder::new().max_rect(max_rect).layout(layout));
    let clip = ui.clip_rect();
    row.set_clip_rect(egui::Rect::from_x_y_ranges(clip.x_range(), max_rect.y_range().intersection(clip.y_range())));
    let inner = add(&mut row);
    let width = row.min_rect().width().max(max_rect.width());
    ui.allocate_rect(egui::Rect::from_min_size(top_left, egui::vec2(width, height)), egui::Sense::hover());
    inner
}

/// The offset that shows `row` whole, as VS Code's explorer reveals a file: scrolled only as much as
/// it needs, below the `covered` rows pinned over the top (the tree's sticky parents), and never past
/// the ends, where the list would spring back and shake. `None`: it is shown already.
///
/// `(row_height, spacing)`: as given to `show_rows`; `rows`: how many there are.
///
/// e.g. rows of 20 px (no spacing), 100 px seen, at 0: row 7 -> `Some(60.0)` (rows 3..=7 seen); row 2
/// -> `None`; with 2 rows covering the top at 60: row 4 -> `Some(40.0)`.
pub(in crate::patch) fn offset_to_show(
    row: usize,
    (row_height, spacing): (f32, f32),
    rows: usize,
    scroll: ListScroll,
    covered: usize,
) -> Option<f32> {
    #[expect(clippy::cast_precision_loss, reason = "far fewer rows than f32 can count")]
    let (row, rows, covered) = (row as f32, rows as f32, covered as f32);
    let step = row_height + spacing;
    let (top, pinned) = (row * step, covered * step);
    let wanted = if top - pinned < scroll.offset {
        top - pinned
    } else if top + row_height > scroll.offset + scroll.view {
        top + row_height - scroll.view
    } else {
        return None;
    };
    // As `show_rows` makes the content: no spacing after the last row.
    let max = rows.mul_add(step, -spacing).max(0.0) - scroll.view;
    let wanted = wanted.min(max).max(0.0);
    ((wanted - scroll.offset).abs() >= 0.5).then_some(wanted)
}

impl PatchTool {
    /// Notes whether the editor `has_focus` this frame. Getting it asks the side bar to show the
    /// object again (next frame: the side bar is drawn first).
    pub(in crate::patch) fn note_editor_focus(&mut self, ctx: &egui::Context, has_focus: bool) {
        if has_focus && !self.pane.had_focus {
            self.explorer.reveal = true;
            ctx.request_repaint();
        }
        self.pane.had_focus = has_focus;
    }

    /// The keyboard goes to the object tree or list, as it shows, its filter kept (Vim `Space e`, or
    /// the palette).
    pub(in crate::patch) fn enter_tree(&mut self, ui: &egui::Ui, editor: egui::Id) {
        // The tree or the list, as it shows: the keys are the same in both.
        self.explorer.tree_focus = true;
        self.side.view = crate::patch::ui::side_bar::SideView::Explorer;
        self.side.hidden = false;
        self.side.objects_closed = false;
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// `Ctrl+Shift+L`, as VS Code's `Ctrl+Shift+M`: the Problems panel shows and gets the keyboard;
    /// if it has it, it hides and the editor gets it back. See [`egui_code_editor::problems`].
    pub(in crate::patch) fn toggle_problems_focus(&mut self, ui: &egui::Ui, editor: egui::Id) {
        let cursor = &mut self.problems.cursor;
        if cursor.focused {
            cursor.focused = false;
            self.problems.open = false;
            self.pane.focus = Some(FocusAt::Keep);
            return;
        }
        self.problems.open = true;
        if self.problems.list.is_none() {
            return; // Not checked yet: no panel to take the keyboard.
        }
        self.problems.cursor.focused = true;
        self.explorer.tree_focus = false;
        ui.memory_mut(|m| m.surrender_focus(editor));
    }

    /// The file object `id` of the active behavior is in, to find it on disk: its patch file in the
    /// mod if this work has one for it (as the export writes it, whether written yet or not), else
    /// the template it is read from.
    ///
    /// e.g. `D:/mods/My/Nemesis_Engine/mod/mymod/0_master/#0106.txt`,
    /// `D:/mods/My/Pandora_Engine/mod/mymod/patches/defaultmale~0_master.xml`,
    /// `<templates>/meshes/actors/character/behaviors/0_master.xml`
    pub(in crate::patch) fn object_file(&self, id: &str) -> std::path::PathBuf {
        let behavior = &self.workspace.behaviors[self.active];
        let patched = behavior.edits.contains_key(id) || behavior.added.contains_key(id);
        let (output, code) = (self.output.trim(), self.mod_code.trim());
        if !patched || output.is_empty() || code.is_empty() {
            return behavior.template_path.clone();
        }
        let output = std::path::Path::new(output);
        let format = self.workspace.export_format;
        if format == crate::patch::workspace::ExportFormat::PandoraV5
            && !behavior.is_anim_data()
            && let Some(file) = behavior.pandora_patch_path(output, code)
        {
            return file;
        }
        output.join(behavior.patch_dir(code, format.is_ext())).join(behavior.patch_file(id))
    }

    /// The keyboard goes back to the editor.
    pub(in crate::patch) const fn leave_tree(&mut self) {
        self.explorer.tree_focus = false;
        self.pane.focus = Some(FocusAt::Keep);
    }

    /// The flat list has the same keys as the tree, without the folding.
    pub(in crate::patch) fn list_keys(&mut self, ui: &egui::Ui, graph: &tree::Graph) {
        let rows = self.explorer.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let selected = self.selected.as_deref();
        let mut current = rows.iter().position(|&node| selected == Some(graph.id(node))).unwrap_or(0);
        let mut leave = false;
        let before = current;
        for input in ui::tree::read_keys(ui, &mut self.explorer.list_leader) {
            leave |= input == tree::TreeInput::Leave;
            current = input.step(current, rows.len()).unwrap_or(current);
        }
        if let Some(&node) = rows.get(current) {
            self.selected = Some(graph.id(node).into());
        }
        if current != before {
            self.explorer.list_scroll_to = Some(current);
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
        let generation = if self.explorer.filter.is_empty() { graph.tree_generation() } else { graph.generation() };
        let fresh = self.explorer.list.as_ref().is_some_and(|list| {
            list.generation == generation
                && list.filter == self.explorer.filter
                && list.changed_only == self.explorer.changed_only
                && list.unlinked_only == self.explorer.unlinked_only
        });
        if fresh {
            return;
        }
        let filter = self.explorer.filter.as_str();
        let rows = graph
            .nodes()
            .filter(|&node| !self.explorer.changed_only || graph.is_changed(node))
            .filter(|&node| !self.explorer.unlinked_only || graph.is_unlinked(node))
            .filter(|&node| contains_ignore_case(graph.label(node), filter))
            .collect();
        // Anim data is grouped by actor: see `ui::anim_tree`.
        let mut rows: Vec<tree::Node> = rows;
        if self.workspace.behaviors[self.active].is_anim_data() {
            rows.sort_by(|&a, &b| cmp_ignore_case(graph.id(a), graph.id(b)));
        }
        self.explorer.list = Some(ObjectList {
            generation,
            filter: self.explorer.filter.clone(),
            changed_only: self.explorer.changed_only,
            unlinked_only: self.explorer.unlinked_only,
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
        self.pane.opened = None;
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

    /// A patched object is in its patch file in the mod; one that isn't, in its template.
    #[test]
    fn the_file_of_an_object() {
        use std::path::Path;

        use crate::patch::workspace::{ADSF, Behavior};
        let mut clips = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        clips.added.insert("Male~1/m$0.txt".to_owned(), String::new());
        let mut tool =
            PatchTool { mod_code: "mymod".to_owned(), output: "D:/mods/My".to_owned(), ..Default::default() };
        tool.workspace.behaviors.push(clips);

        let patch = Path::new("D:/mods/My").join("Nemesis_Engine/mod/mymod").join(ADSF).join("Male~1/m$0.txt");
        assert_eq!(tool.object_file("Male~1/m$0.txt"), patch);
        let template = tool.workspace.behaviors[0].template_path.clone();
        assert_eq!(tool.object_file("Male~1/other.txt"), template, "not patched");
        tool.output.clear();
        assert_eq!(tool.object_file("Male~1/m$0.txt"), template, "no mod to be in");
    }

    #[test]
    fn a_list_scrolls_only_when_the_editor_gets_the_keyboard() {
        let a: Rc<str> = "#a".into();
        assert!(!take_reveal(Some(&a), false), "a selection alone");
        assert!(take_reveal(Some(&a), true), "the editor got the keyboard");
        assert!(!take_reveal(None, true), "nothing to show");
    }

    /// Only as much as the row needs, below what is pinned, and never past the ends.
    #[test]
    fn a_row_is_scrolled_into_view() {
        let at = |offset| ListScroll { offset, view: 100.0 };
        assert_eq!(offset_to_show(7, (20.0, 0.0), 50, at(0.0), 0), Some(60.0), "below: it comes to the bottom");
        assert_eq!(offset_to_show(2, (20.0, 0.0), 50, at(0.0), 0), None, "seen already");
        assert_eq!(offset_to_show(1, (20.0, 0.0), 50, at(60.0), 0), Some(20.0), "above: it comes to the top");
        assert_eq!(offset_to_show(4, (20.0, 0.0), 50, at(60.0), 2), Some(40.0), "under the pinned rows");
        assert_eq!(offset_to_show(0, (20.0, 0.0), 50, at(60.0), 2), Some(0.0), "not above the start");
        // 10 rows of 20 px with 4 px between: 236 px, 136 px at most.
        assert_eq!(offset_to_show(9, (20.0, 4.0), 10, at(0.0), 0), Some(136.0), "not past the end");
        assert_eq!(offset_to_show(9, (20.0, 4.0), 10, at(136.0), 0), None, "at the end already");
    }

    /// Only getting the keyboard asks: keeping it does not, or the side bar could not be scrolled.
    #[test]
    fn getting_into_the_editor_asks_the_side_bar_to_follow() {
        let ctx = egui::Context::default();
        let mut tool = PatchTool::default();
        tool.note_editor_focus(&ctx, true);
        assert!(std::mem::take(&mut tool.explorer.reveal));
        tool.note_editor_focus(&ctx, true);
        assert!(!tool.explorer.reveal);
        tool.note_editor_focus(&ctx, false);
        tool.note_editor_focus(&ctx, true);
        assert!(tool.explorer.reveal);
    }
}
