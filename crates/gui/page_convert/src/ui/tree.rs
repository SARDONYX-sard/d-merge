//! Draws the checkbox tree. The check state model is in [`crate::tree`].

use egui_widgets::shadcn::Checkbox;
use fast_hash::HashSet;
use serde_hkx_for_gui::{DirEntry, status::Status};

use crate::tree::{TreeView, count_files, for_each_dir, for_each_file, toggle};

impl TreeView {
    /// Draws the tree. `status_of` returns the conversion status for a file path.
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, status_of: &dyn Fn(&str) -> Option<Status>) {
        let Self { roots, checked, expanded, .. } = self;
        let mut state = NodeState { checked, expanded, status_of };
        for root in roots.iter() {
            node_ui(ui, root, 0, &mut state);
        }
    }
}

/// Mutable pieces needed while drawing nodes recursively.
struct NodeState<'a> {
    checked: &'a mut HashSet<String>,
    expanded: &'a mut HashSet<String>,
    status_of: &'a dyn Fn(&str) -> Option<Status>,
}

/// One row (+ its children when expanded).
fn node_ui(ui: &mut egui::Ui, entry: &DirEntry, depth: usize, state: &mut NodeState) {
    const INDENT: f32 = 18.0;

    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * INDENT);

        if entry.children.is_some() {
            directory_row(ui, entry, state);
        } else {
            file_row(ui, entry, state);
        }
    });
    if state.expanded.contains(&entry.path) {
        for child in entry.children.iter().flatten() {
            node_ui(ui, child, depth + 1, state);
        }
    }
}

/// `[▼] [✔/-/ ] name`: expand button + tri-state checkbox.
fn directory_row(ui: &mut egui::Ui, entry: &DirEntry, state: &mut NodeState) {
    let is_expanded = state.expanded.contains(&entry.path);
    if ui.small_button(if is_expanded { "▼" } else { "▶" }).clicked() {
        toggle(state.expanded, &entry.path);
    }

    let (total, checked) = count_files(entry, state.checked);
    let mut all = total > 0 && checked == total;
    let partial = checked > 0 && checked < total;
    if ui.add(Checkbox::without_text(&mut all).indeterminate(partial)).clicked() {
        // Clicking a full/partial directory clears it; an empty one checks everything.
        let check = checked == 0;
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
    }
    super::status_icon(ui, (state.status_of)(&entry.path));
    ui.label(&entry.name);
}
