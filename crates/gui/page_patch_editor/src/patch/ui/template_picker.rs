//! Draws the template picker: a directory tree, or a flat list while filtering.
//! The rows of the tree are computed in [`crate::patch::template_tree`].

use std::path::{Path, PathBuf};

use egui_widgets::shadcn::TextEdit;
use fast_hash::HashSet;
use gui_shared::i18n::{I18nKey, I18nMap};
use stdx::text::contains_ignore_case;

use crate::patch::{
    template_tree::{self, RowKind},
    workspace::{TemplateEntry, TemplateScope},
};

/// The window's state: its own only.
#[derive(Debug, Default)]
pub(super) struct TemplatePicker {
    is_open: bool,
    /// The filter takes the keyboard on the next draw.
    focus: bool,
    /// Templates found in `template_dir` that the work's scope allows.
    catalog: Vec<TemplateEntry>,
    /// `(template_dir, scope)` of `catalog`.
    catalog_for: Option<(String, TemplateScope)>,
    filter: String,
    /// The directories of the tree flipped from how they start. See [`template_tree::toggle`].
    toggled: HashSet<String>,
}

impl TemplatePicker {
    /// Shows the window, its filter with the keyboard.
    pub(super) const fn open(&mut self) {
        self.is_open = true;
        self.focus = true;
    }

    /// Draws the window, if open: the templates of `template_dir` that `scope` allows. Returns the
    /// one clicked (the window closes then).
    pub(super) fn window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        (template_dir, scope): (&str, TemplateScope),
    ) -> Option<PathBuf> {
        if !self.is_open {
            return None;
        }
        self.refresh(template_dir, scope);

        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let window = egui_widgets::quick_pick_window(ctx, i18n.t(I18nKey::NemesisAddBehavior)).open(&mut open);
        let picked =
            window.show(ctx, |ui| self.contents(ui, i18n, (template_dir, scope))).and_then(|shown| shown.inner);
        let picked = picked.flatten();

        self.is_open = open && picked.is_none();
        picked
    }

    /// The catalog again, if the directory or the scope changed since.
    fn refresh(&mut self, template_dir: &str, scope: TemplateScope) {
        let fresh = self.catalog_for.as_ref().is_some_and(|(dir, of)| dir == template_dir && *of == scope);
        if !fresh {
            self.catalog = crate::patch::workspace::catalog(Path::new(template_dir), scope);
            self.catalog_for = Some((template_dir.to_owned(), scope));
        }
    }

    /// The filter, where the templates are, and the tree (or, filtering, the matches).
    fn contents(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (template_dir, scope): (&str, TemplateScope),
    ) -> Option<PathBuf> {
        let field = TextEdit::singleline(&mut self.filter)
            .hint_text(i18n.t(I18nKey::NemesisFilterHint))
            .desired_width(ui.available_width());
        let field = ui.add(field);
        if std::mem::take(&mut self.focus) {
            field.request_focus();
        }

        ui.weak(format!("{template_dir} ({})", self.catalog.len()));
        if scope == TemplateScope::Nemesis {
            ui.weak(i18n.t(I18nKey::NemesisScopeNemesisOnly));
        }
        if self.catalog.is_empty() {
            ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNoTemplates));
            return None;
        }

        // No filter: as a directory tree. A filter wants all the matches at once.
        let clicked = if self.filter.is_empty() {
            tree_ui(ui, &self.catalog, &mut self.toggled, i18n)
        } else {
            filtered_ui(ui, &self.catalog, &self.filter, i18n)
        };
        clicked.map(Path::to_path_buf)
    }
}

const MAX_HEIGHT: f32 = 420.0;
/// Pixels of indent per directory level.
const INDENT: f32 = 12.0;

/// The tree. Returns the template clicked; a clicked directory flips its entry in `toggled`.
fn tree_ui<'a>(
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
fn filtered_ui<'a>(ui: &mut egui::Ui, catalog: &'a [TemplateEntry], filter: &str, i18n: &I18nMap) -> Option<&'a Path> {
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
    let shown: Vec<&TemplateEntry> = catalog.iter().filter(|e| contains_ignore_case(&e.meshes_path, filter)).collect();
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
