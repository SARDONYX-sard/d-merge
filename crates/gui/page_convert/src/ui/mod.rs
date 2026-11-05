//! Drawing of the Convert page: inputs, output and the run button. The logic is in the modules next
//! to this directory.

mod tree;

use std::path::{Path, PathBuf};

use egui_widgets::shadcn::{Button, Icon, TextEdit};
use gui_shared::{
    fs::open_existing_dir_or_ancestor,
    i18n::{I18nKey, I18nMap},
    settings::pages::convert::{OutputFormat, SelectionType},
};
use serde_hkx_for_gui::status::Status;

use crate::{
    ConvertPage, ConvertPageCtx, INPUT_EXTENSIONS, add_dropped, add_paths, migrate_dir_mode, task::ConvertTask,
};

impl ConvertPage {
    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, mut ctx: ConvertPageCtx<'_>) {
        migrate_dir_mode(ctx.settings);
        handle_drop(ui.ctx(), ctx.settings);

        // NOTE: Bottom panel first, then the central area (egui panel order).
        egui::Panel::bottom("convert_actions").frame(ctx.panel_frame).show(ui, |ui| self.output_ui(ui, &mut ctx));

        egui::CentralPanel::default().frame(ctx.panel_frame).show(ui, |ui| {
            self.inputs_header(ui, &mut ctx);
            ui.add_space(4.0);
            self.inputs_toolbar(ui, &mut ctx);
            ui.add_space(4.0);
            self.inputs(ui, &mut ctx);
        });

        drop_overlay(ui.ctx(), ctx.i18n);
    }

    /// `Inputs                [Files & Directories] [Directory Tree]`
    fn inputs_header(&self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        ui.horizontal(|ui| {
            ui.heading(ctx.i18n.t(I18nKey::ConvertInputsLabel));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Right-to-left: the last tab first.
                let tabs = [
                    (SelectionType::Tree, I18nKey::ConvertSelectionTree, I18nKey::ConvertSelectionTreeHover),
                    (SelectionType::Files, I18nKey::ConvertSelectionFiles, I18nKey::ConvertSelectionFilesHover),
                ];
                for (kind, label, hover) in tabs {
                    let current = &mut ctx.settings.selection_type;
                    if ui
                        .add(Button::selectable(*current == kind, ctx.i18n.t(label)))
                        .on_hover_text(ctx.i18n.t(hover))
                        .clicked()
                        && *current != kind
                    {
                        *current = kind;
                        // Statuses are keyed by path; clear them so modes do not mix.
                        self.task.clear_statuses();
                    }
                }
            });
        });
    }

    /// `[+ Add Files…] [+ Add Directories…]   [Clear]` (Tree: `[+ Add Root Directories…] [Select All] ...`)
    fn inputs_toolbar(&mut self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        let i18n = ctx.i18n;
        ui.horizontal(|ui| {
            let settings = &mut *ctx.settings;
            match settings.selection_type {
                SelectionType::Files | SelectionType::Dir => {
                    let add_files = Button::new(i18n.t(I18nKey::ConvertAddFiles)).icon(Icon::FilePlus);
                    if ui.add(add_files).clicked()
                        && let Some(files) =
                            dialog(&settings.selected_files).add_filter("Havok", &INPUT_EXTENSIONS).pick_files()
                    {
                        add_paths(&mut settings.selected_files, files.iter().map(|p| p.as_path()));
                    }
                    let add_dirs = Button::new(i18n.t(I18nKey::ConvertAddDirs)).icon(Icon::FolderPlus);
                    if ui.add(add_dirs).clicked()
                        && let Some(dirs) = dialog(&settings.selected_files).pick_folders()
                    {
                        add_paths(&mut settings.selected_files, dirs.iter().map(|p| p.as_path()));
                    }
                }
                SelectionType::Tree => {
                    let add_roots = Button::new(i18n.t(I18nKey::ConvertAddRootDirs)).icon(Icon::FolderPlus);
                    if ui.add(add_roots).clicked()
                        && let Some(dirs) = dialog(&settings.tree_roots).pick_folders()
                    {
                        add_paths(&mut settings.tree_roots, dirs.iter().map(|p| p.as_path()));
                    }

                    let tree = &mut self.tree;
                    let select_label =
                        if tree.has_checked() { I18nKey::ConvertUnselectAll } else { I18nKey::ConvertSelectAll };
                    if ui.add(Button::new(i18n.t(select_label))).clicked() {
                        tree.toggle_select_all();
                    }
                    let expand_label =
                        if tree.is_any_expanded() { I18nKey::CollapseAllButton } else { I18nKey::ExpandAllButton };
                    if ui.add(Button::new(i18n.t(expand_label))).clicked() {
                        tree.toggle_expand_all();
                    }
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clear = Button::new(i18n.t(I18nKey::ConvertClearButton)).icon(Icon::Trash);
                if ui.add_enabled(self.input_count(settings) > 0, clear).clicked() {
                    self.clear_inputs(settings);
                }
            });
        });
    }

    /// The list (Files & Directories) or the checkbox tree, in a framed box; a drop hint when empty.
    fn inputs(&mut self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        if ctx.settings.selection_type == SelectionType::Tree
            && let Err(err) = self.tree.ensure_loaded(&ctx.settings.tree_roots)
        {
            ctx.notify.error(err);
        }

        let empty = match ctx.settings.selection_type {
            SelectionType::Tree => ctx.settings.tree_roots.is_empty(),
            SelectionType::Files | SelectionType::Dir => ctx.settings.selected_files.is_empty(),
        };
        if empty {
            drop_zone(ui, ctx.i18n.t(I18nKey::ConvertDropHint));
            return;
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let task = &self.task;
            match ctx.settings.selection_type {
                SelectionType::Tree => {
                    // `✖` per root, then the tree, which scrolls by itself.
                    ctx.settings.tree_roots.retain(|root| {
                        ui.horizontal(|ui| {
                            ui.add(Icon::FolderOpen);
                            ui.strong(root);
                            !remove_button(ui)
                        })
                        .inner
                    });
                    ui.separator();
                    self.tree.ui(ui, &|path| task.status_of(path));
                }
                SelectionType::Files | SelectionType::Dir => {
                    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        ctx.settings.selected_files.retain(|path| path_row(ui, path, task));
                    });
                }
            }
        });
    }

    /// Format, directory and the Convert button: the action next to what it needs.
    fn output_ui(&mut self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        ui.add_space(6.0);
        egui::Grid::new("convert_output").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
            format_row(ui, ctx);
            output_row(ui, ctx);
        });
        ui.add_space(6.0);

        let running = self.task.is_running();
        let count = self.input_count(ctx.settings);
        let label = if running {
            ctx.i18n.t(I18nKey::ConvertingButton).to_string()
        } else {
            format!("{} ({count}) → {}", ctx.i18n.t(I18nKey::ConvertButton), ctx.settings.format.label())
        };
        let button = Button::new(label).primary().full_width().min_size(egui::vec2(0.0, 40.0));
        let response = ui.add_enabled(!running && count > 0, button);
        let response =
            if count == 0 { response.on_disabled_hover_text(ctx.i18n.t(I18nKey::ConvertNoInput)) } else { response };
        if response.clicked() {
            self.start_convert(ui.ctx(), ctx);
        }
        ui.add_space(4.0);
    }
}

/// Adds files/directories dropped onto the window (only while this page is shown).
fn handle_drop(egui_ctx: &egui::Context, settings: &mut gui_shared::settings::pages::convert::ConvertSettings) {
    let dropped: Vec<PathBuf> =
        egui_ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
    if !dropped.is_empty() {
        add_dropped(settings, &dropped);
    }
}
/// `Output format: [HKX SE (64-bit)] [HKX LE (32-bit)] [XML] [JSON]`
fn format_row(ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
    ui.label(ctx.i18n.t(I18nKey::ConvertOutputFormatLabel));
    ui.horizontal(|ui| {
        let format = &mut ctx.settings.format;
        for kind in OutputFormat::ALL {
            if ui.add(Button::selectable(*format == kind, kind.label())).clicked() {
                *format = kind;
            }
        }
    });
    ui.end_row();
}

/// `Output: [Next to each input] [Select] [📂]`
fn output_row(ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
    ui.label(ctx.i18n.t(I18nKey::ConvertOutputLabel)).on_hover_text(ctx.i18n.t(I18nKey::ConvertOutputHover));

    let output = &mut ctx.settings.output;
    // Right-to-left: the buttons take their width first, the text edit gets the rest.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let open = Button::new("").icon(Icon::FolderOpen);
        if ui.add_enabled(!output.is_empty(), open).on_hover_text(ctx.i18n.t(I18nKey::ConvertOpenOutputHover)).clicked()
            && let Err(err) = open_existing_dir_or_ancestor(Path::new(output.as_str()))
        {
            tracing::error!(err);
        }
        if ui.add(Button::new(ctx.i18n.t(I18nKey::SelectButton))).clicked()
            && let Some(dir) = gui_shared::rfd::FileDialog::new().set_directory(output.as_str()).pick_folder()
        {
            *output = dir.display().to_string();
        }
        let width = ui.available_width();
        ui.add(
            TextEdit::singleline(output).hint_text(ctx.i18n.t(I18nKey::ConvertOutputPlaceholder)).desired_width(width),
        )
        .on_hover_text(ctx.i18n.t(I18nKey::ConvertOutputHover));
    });
    ui.end_row();
}

/// `📄 path   (status)  ✖` -> `false` when removed.
fn path_row(ui: &mut egui::Ui, path: &str, task: &ConvertTask) -> bool {
    ui.horizontal(|ui| {
        let is_dir = Path::new(path).is_dir();
        ui.add(if is_dir { Icon::FolderOpen } else { Icon::FilePlus });
        ui.label(path);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let remove = remove_button(ui);
            status_icon(ui, task.status_of(path));
            !remove
        })
        .inner
    })
    .inner
}

fn remove_button(ui: &mut egui::Ui) -> bool {
    ui.add(egui::Button::new("✖").frame(false).small()).clicked()
}

/// Empty input box: a dashed drop target with the hint in the middle.
fn drop_zone(ui: &mut egui::Ui, hint: &str) {
    let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
    let color = ui.visuals().weak_text_color();
    let r = rect.shrink(2.0);
    let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    let stroke = egui::Stroke::new(1.5_f32, color);
    ui.painter().extend(egui::Shape::dashed_line(&corners, stroke, 8.0, 6.0));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, hint, egui::FontId::proportional(18.0), color);
}

/// Blue overlay while files are dragged over the window.
fn drop_overlay(ctx: &egui::Context, i18n: &I18nMap) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("convert_drop"));
    let painter = ctx.layer_painter(layer);
    let rect = ctx.content_rect();
    let blue = egui::Color32::from_rgb(66, 165, 245);
    painter.rect_filled(rect, 0.0, blue.gamma_multiply(0.15));
    painter.rect_stroke(rect.shrink(4.0), 8.0, egui::Stroke::new(3.0_f32, blue), egui::StrokeKind::Inside);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        i18n.t(I18nKey::ConvertDropHint),
        egui::FontId::proportional(24.0),
        blue,
    );
}

/// File dialog starting at the first of `paths` (or its parent for a file).
fn dialog(paths: &[String]) -> gui_shared::rfd::FileDialog {
    let start = paths.first().map(Path::new);
    let start = start.and_then(|p| if p.is_dir() { Some(p) } else { p.parent() });
    gui_shared::rfd::FileDialog::new().set_directory(start.unwrap_or_else(|| Path::new("")))
}

/// The state of a file in a list or tree: spinning, `✔` or `✖`.
pub(crate) fn status_icon(ui: &mut egui::Ui, status: Option<Status>) {
    match status {
        Some(Status::Processing) => {
            ui.add(egui::Spinner::new().size(14.0));
        }
        Some(Status::Done) => {
            ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔");
        }
        Some(Status::Error) => {
            ui.colored_label(ui.visuals().error_fg_color, "✖");
        }
        Some(Status::Pending) | None => {}
    }
}
