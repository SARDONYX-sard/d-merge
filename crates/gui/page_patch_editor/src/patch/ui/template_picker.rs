//! Draws the template picker: a directory tree, or a flat list while filtering.
//! The rows of the tree are computed in [`crate::patch::template_tree`].

use std::path::Path;

use fast_hash::HashSet;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    template_tree::{self, RowKind},
    workspace::TemplateEntry,
};

const MAX_HEIGHT: f32 = 420.0;
/// Pixels of indent per directory level.
const INDENT: f32 = 12.0;

/// The tree. Returns the template clicked; a clicked directory flips its entry in `toggled`.
pub(in crate::patch) fn tree_ui<'a>(
    ui: &mut egui::Ui,
    catalog: &'a [TemplateEntry],
    toggled: &mut HashSet<String>,
    i18n: &I18nMap,
) -> Option<&'a Path> {
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    let rows = template_tree::rows(catalog, toggled);
    let mut picked = None;
    let mut toggle = None;
    egui::ScrollArea::vertical().max_height(MAX_HEIGHT).auto_shrink([false, true]).show_rows(
        ui,
        row_height,
        rows.len(),
        |ui, range| {
            for row in &rows[range] {
                ui.horizontal(|ui| {
                    ui.add_space(f32::from(row.depth) * INDENT);
                    match row.kind {
                        RowKind::Dir { path, open } => {
                            let glyph = if open { "▼" } else { "▶" };
                            let label = egui::RichText::new(format!("{glyph} {}", row.name)).monospace();
                            if ui.add(egui::Button::selectable(false, label)).clicked() {
                                toggle = Some(path);
                            }
                        }
                        RowKind::File(entry) => {
                            if file_button(ui, row.name, entry, i18n) {
                                picked = Some(entry.path.as_path());
                            }
                        }
                    }
                });
            }
        },
    );
    if let Some(path) = toggle {
        template_tree::toggle(toggled, path);
    }
    picked
}

/// The flat list of the templates whose path contains `filter`. Returns the template clicked.
pub(in crate::patch) fn filtered_ui<'a>(
    ui: &mut egui::Ui,
    catalog: &'a [TemplateEntry],
    filter: &str,
    i18n: &I18nMap,
) -> Option<&'a Path> {
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    let filter = filter.to_lowercase();
    let shown: Vec<&TemplateEntry> = catalog.iter().filter(|e| e.meshes_path.contains(&filter)).collect();
    let mut picked = None;
    egui::ScrollArea::vertical().max_height(MAX_HEIGHT).auto_shrink([false, true]).show_rows(
        ui,
        row_height,
        shown.len(),
        |ui, range| {
            for entry in &shown[range] {
                ui.horizontal(|ui| {
                    if file_button(ui, &entry.meshes_path, entry, i18n) {
                        picked = Some(entry.path.as_path());
                    }
                });
            }
        },
    );
    picked
}

/// `path  Nemesis` or a warned `path  Ext` (only d-merge reads these). Returns "clicked".
fn file_button(ui: &mut egui::Ui, label: &str, entry: &TemplateEntry, i18n: &I18nMap) -> bool {
    let clicked = ui.add(egui::Button::selectable(false, egui::RichText::new(label).monospace())).clicked();
    if entry.nemesis {
        ui.weak("Nemesis");
    } else {
        ui.colored_label(ui.visuals().warn_fg_color, "Ext").on_hover_text(i18n.t(I18nKey::NemesisExtOnlyWarning));
    }
    clicked
}
