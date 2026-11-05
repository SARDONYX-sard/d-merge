//! Draws the checkbox tree. The check state model is in [`crate::tree`].

use egui_widgets::shadcn::Checkbox;
use fast_hash::{HashMap, HashSet};
use serde_hkx_for_gui::{DirEntry, status::Status};

use crate::tree::{TreeView, for_each_dir, for_each_file, toggle, visible_rows};

impl TreeView {
    /// Draws the tree. `status_of` returns the conversion status for a file path.
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status_of: &dyn Fn(&str) -> Option<Status>) {
        self.counts(); // computed now, so that the rows can read them while editing `checked`
        let Self { roots, checked, expanded, counts, .. } = self;
        let Some(counts) = counts.as_ref() else { return };
        let mut state = NodeState { checked, expanded, status_of, counts, checks_changed: false };
        // Only the rows on screen are laid out: a tree may have thousands of files.
        let rows = visible_rows(roots, state.expanded);
        let row_height = ui.spacing().interact_size.y + 2.0;
        egui::ScrollArea::vertical().auto_shrink(false).show_rows(ui, row_height, rows.len(), |ui, range| {
            for (entry, depth) in &rows[range] {
                row_ui(ui, entry, *depth, row_height, &mut state);
            }
        });
        if state.checks_changed {
            self.invalidate_counts();
        }
    }
}

/// Mutable pieces needed while drawing nodes recursively.
struct NodeState<'a> {
    checked: &'a mut HashSet<String>,
    expanded: &'a mut HashSet<String>,
    status_of: &'a dyn Fn(&str) -> Option<Status>,
    /// See `TreeView::counts`.
    counts: &'a HashMap<String, (u32, u32)>,
    /// A box was clicked: the counts must be recomputed.
    checks_changed: bool,
}

/// One row.
fn row_ui(ui: &mut egui::Ui, entry: &DirEntry, depth: usize, height: f32, state: &mut NodeState) {
    const INDENT: f32 = 18.0;

    ui.horizontal(|ui| {
        ui.set_height(height);
        #[expect(clippy::cast_precision_loss, reason = "a small depth")]
        ui.add_space(depth as f32 * INDENT);
        if entry.children.is_some() {
            directory_row(ui, entry, state);
        } else {
            file_row(ui, entry, state);
        }
    });
}

/// `[▼] [✔/-/ ] name`: expand button + tri-state checkbox.
fn directory_row(ui: &mut egui::Ui, entry: &DirEntry, state: &mut NodeState) {
    let is_expanded = state.expanded.contains(&entry.path);
    if ui.small_button(if is_expanded { "▼" } else { "▶" }).clicked() {
        toggle(state.expanded, &entry.path);
    }

    let (total, checked) = state.counts.get(&entry.path).copied().unwrap_or_default();
    let mut all = total > 0 && checked == total;
    let partial = checked > 0 && checked < total;
    if ui.add(Checkbox::without_text(&mut all).indeterminate(partial)).clicked() {
        // Clicking a full/partial directory clears it; an empty one checks everything.
        let check = checked == 0;
        state.checks_changed = true;
        for_each_file(entry, &mut |file| {
            if check {
                state.checked.insert(file.path.clone());
            } else {
                state.checked.remove(&file.path);
            }
        });
    }
    // Right click: open / close this directory and everything below it.
    ui.label(&entry.name).context_menu(|ui| {
        for (open, label) in [(true, "⊞ Expand all below"), (false, "⊟ Collapse all below")] {
            if ui.button(label).clicked() {
                for_each_dir(entry, &mut |dir| {
                    if open {
                        state.expanded.insert(dir.path.clone());
                    } else {
                        state.expanded.remove(&dir.path);
                    }
                });
            }
        }
    });
}

/// `[✔] (status) name`
fn file_row(ui: &mut egui::Ui, entry: &DirEntry, state: &mut NodeState) {
    // Align with directories, which have an expand button.
    ui.add_space(ui.spacing().interact_size.y);

    let mut is_checked = state.checked.contains(&entry.path);
    if ui.add(Checkbox::without_text(&mut is_checked)).changed() {
        toggle(state.checked, &entry.path);
        state.checks_changed = true;
    }
    super::status_icon(ui, (state.status_of)(&entry.path));
    ui.label(&entry.name);
}
