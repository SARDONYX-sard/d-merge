//! Convert page: hkx <-> xml/json converter.
//!
//! ```text
//! [ All Clear                                                     ]
//! Output:          [ D:/out                       ] [Select]
//! Selection type:  (•) Directories ( ) Tree ( ) Files  [Select]
//! Output format:   [ Amd64 ▼ ]
//! ─────────────────────────────────────────────────────────────────
//! inputs (tree with checkboxes, or a list of paths) + per-file status
//! ─────────────────────────────────────────────────────────────────
//! [ Convert                                                       ]
//! ```
//!
//! Persisted inputs live in `settings.convert`; transient state (loaded tree, progress) in [`ConvertPage`].

mod task;
mod tree;

use d_merge_gui_shared::{
    fs::open_existing_dir_or_ancestor,
    i18n::I18nKey,
    settings::convert::{OutFormat, SelectionType},
};

pub(crate) use self::task::to_format;
use self::{task::ConvertTask, tree::TreeView};
use crate::{
    app::App,
    theme::{themed_central_panel, themed_top_bottom_panel},
    ui::shadcn::{Button, ComboBox, RadioButton, TextEdit},
};

/// Extensions the converter accepts as input.
const INPUT_EXTENSIONS: [&str; 4] = ["hkx", "xml", "json", "yaml"];

/// Transient Convert page state (see the module docs).
#[derive(Default)]
pub(crate) struct ConvertPage {
    tree: TreeView,
    task: ConvertTask,
}

impl App {
    /// Draws the Convert page.
    pub(crate) fn ui_convert_page(&mut self, ui: &mut egui::Ui) {
        let theme = self.settings.ui.theme;
        let bg = self.theme_manager.current_bg_color().cloned();

        // NOTE: Bottom panel first, then the central area (egui panel order).
        themed_top_bottom_panel(egui::Panel::bottom("convert_actions"), theme, bg.as_ref())
            .show(ui, |ui| self.ui_convert_button(ui));

        themed_central_panel(egui::CentralPanel::default(), theme, bg.as_ref()).show(ui, |ui| {
            if ui
                .add(Button::new(self.i18n.t(I18nKey::ConvertAllClearButton)).full_width())
                .clicked()
            {
                self.convert_clear_all();
            }
            ui.add_space(8.0);

            egui::Grid::new("convert_form").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
                self.ui_convert_output_row(ui);
                self.ui_convert_selection_row(ui);
                self.ui_convert_format_row(ui);
            });

            ui.separator();
            self.ui_convert_inputs(ui);
        });
    }

    /// Called every frame (see `App::frame_begin`): reports a finished conversion.
    pub(crate) fn poll_convert_result(&mut self) {
        match self.convert.task.take_finished() {
            Some(Ok(())) => self.notify.success(self.i18n.t(I18nKey::ConvertDone)),
            Some(Err(err)) => {
                tracing::error!("{err}");
                self.notify.error(err);
            }
            None => {}
        }
    }

    // ── Form rows ────────────────────────────────────────────────────────────

    /// `Output: [path] [Select]` (the label button opens the directory).
    fn ui_convert_output_row(&mut self, ui: &mut egui::Ui) {
        let output = &mut self.settings.convert.output;

        if ui
            .add(Button::new(self.i18n.t(I18nKey::ConvertOutputLabel)))
            .on_hover_text(self.i18n.t(I18nKey::ConvertOutputHover))
            .clicked()
            && let Err(err) = open_existing_dir_or_ancestor(std::path::Path::new(output.as_str()))
        {
            tracing::error!(err);
        }

        // Right-to-left: the button takes its width first, the text edit gets the rest.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui.add(Button::new(self.i18n.t(I18nKey::SelectButton))).clicked()
                && let Some(dir) =
                    rfd::FileDialog::new().set_directory(output.as_str()).pick_folder()
            {
                *output = dir.display().to_string();
            }
            let width = ui.available_width();
            ui.add(TextEdit::singleline(output).desired_width(width));
        });
        ui.end_row();
    }

    /// `Selection type: (•) Directories ( ) Tree ( ) Files [Select]`
    fn ui_convert_selection_row(&mut self, ui: &mut egui::Ui) {
        ui.label(self.i18n.t(I18nKey::ConvertSelectionTypeLabel));

        ui.horizontal(|ui| {
            let options = [
                (
                    SelectionType::Dir,
                    I18nKey::ConvertSelectionDirs,
                    I18nKey::ConvertSelectionDirsHover,
                ),
                (
                    SelectionType::Tree,
                    I18nKey::ConvertSelectionTree,
                    I18nKey::ConvertSelectionTreeHover,
                ),
                (
                    SelectionType::Files,
                    I18nKey::ConvertSelectionFiles,
                    I18nKey::ConvertSelectionFilesHover,
                ),
            ];
            for (kind, label, hover) in options {
                let current = &mut self.settings.convert.selection_type;
                if ui
                    .add(RadioButton::new(*current == kind, self.i18n.t(label)))
                    .on_hover_text(self.i18n.t(hover))
                    .clicked()
                    && *current != kind
                {
                    *current = kind;
                    // Statuses are keyed by path; clear them so modes do not mix.
                    self.convert.task.clear_statuses();
                }
            }

            ui.add_space(8.0);
            if ui.add(Button::new(self.i18n.t(I18nKey::SelectButton))).clicked() {
                self.convert_pick_inputs();
            }
        });
        ui.end_row();
    }

    /// `Output format: [Amd64 ▼]`
    fn ui_convert_format_row(&mut self, ui: &mut egui::Ui) {
        ui.label(self.i18n.t(I18nKey::ConvertOutputFormatLabel));

        let format = &mut self.settings.convert.format;
        let mut index = OutFormat::ALL.iter().position(|f| f == format).unwrap_or(0);
        if ComboBox::from_id_salt("convert_out_format")
            .width(120.0)
            .show_index(ui, &mut index, OutFormat::ALL.len(), |i| OutFormat::ALL[i].label())
            .changed()
        {
            *format = OutFormat::ALL[index];
        }
        ui.end_row();
    }

    // ── Inputs ───────────────────────────────────────────────────────────────

    /// Tree (Tree mode) or a removable list of paths (Files / Directories mode).
    fn ui_convert_inputs(&mut self, ui: &mut egui::Ui) {
        if self.settings.convert.selection_type == SelectionType::Tree {
            if let Err(err) = self.convert.tree.ensure_loaded(&self.settings.convert.tree_roots) {
                self.notify.error(err);
            }

            ui.horizontal(|ui| {
                let tree = &mut self.convert.tree;
                let select_label = if tree.has_checked() {
                    I18nKey::ConvertUnselectAll
                } else {
                    I18nKey::ConvertSelectAll
                };
                if ui.add(Button::new(self.i18n.t(select_label))).clicked() {
                    tree.toggle_select_all();
                }

                let expand_label = if tree.is_any_expanded() {
                    I18nKey::ConvertCollapseAll
                } else {
                    I18nKey::ConvertExpandAll
                };
                if ui.add(Button::new(self.i18n.t(expand_label))).clicked() {
                    tree.toggle_expand_all();
                }
            });

            let task = &self.convert.task;
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                self.convert.tree.ui(ui, &|path| task.status_of(path));
            });
            return;
        }

        // Files / Directories: `(status) path [✖]`
        let paths = match self.settings.convert.selection_type {
            SelectionType::Dir => &mut self.settings.convert.selected_dirs,
            SelectionType::Files | SelectionType::Tree => &mut self.settings.convert.selected_files,
        };
        let task = &self.convert.task;
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            paths.retain(|path| {
                ui.horizontal(|ui| {
                    let remove = ui.small_button("✖").clicked();
                    task::status_icon(ui, task.status_of(path));
                    ui.label(path);
                    !remove
                })
                .inner
            });
        });
    }

    /// Opens the file/folder dialog matching the selection mode.
    fn convert_pick_inputs(&mut self) {
        let convert = &mut self.settings.convert;
        let start_dir = match convert.selection_type {
            SelectionType::Files => convert.selected_files.first(),
            SelectionType::Dir => convert.selected_dirs.first(),
            SelectionType::Tree => convert.tree_roots.first(),
        }
        .cloned()
        .unwrap_or_default();
        let dialog = rfd::FileDialog::new().set_directory(&start_dir);

        let to_strings = |paths: Vec<std::path::PathBuf>| -> Vec<String> {
            paths.iter().map(|p| p.display().to_string()).collect()
        };

        let picked = match convert.selection_type {
            SelectionType::Files => dialog
                .add_filter("Havok", &INPUT_EXTENSIONS)
                .pick_files()
                .map(|files| (&mut convert.selected_files, to_strings(files))),
            SelectionType::Dir => {
                dialog.pick_folders().map(|dirs| (&mut convert.selected_dirs, to_strings(dirs)))
            }
            SelectionType::Tree => {
                dialog.pick_folders().map(|dirs| (&mut convert.tree_roots, to_strings(dirs)))
            }
        };

        if let Some((target, paths)) = picked {
            *target = paths;
            self.convert.task.clear_statuses();
        }
    }

    /// "All Clear": forget the inputs of the current mode, the output and the statuses.
    fn convert_clear_all(&mut self) {
        let convert = &mut self.settings.convert;
        match convert.selection_type {
            SelectionType::Files => convert.selected_files.clear(),
            SelectionType::Dir => convert.selected_dirs.clear(),
            SelectionType::Tree => {
                convert.tree_roots.clear();
                self.convert.tree.clear();
            }
        }
        convert.output.clear();
        self.convert.task.clear_statuses();
    }

    // ── Run ──────────────────────────────────────────────────────────────────

    fn ui_convert_button(&mut self, ui: &mut egui::Ui) {
        let running = self.convert.task.is_running();
        let label = if running {
            self.i18n.t(I18nKey::ConvertingButton)
        } else {
            self.i18n.t(I18nKey::ConvertButton)
        };

        ui.add_space(4.0);
        let button = Button::new(label).primary().full_width().min_size(egui::vec2(0.0, 40.0));
        if ui.add_enabled(!running, button).clicked() {
            self.start_convert(ui.ctx());
        }
        ui.add_space(4.0);
    }

    fn start_convert(&mut self, ctx: &egui::Context) {
        let convert = &self.settings.convert;
        let (inputs, roots) = match convert.selection_type {
            SelectionType::Files => (convert.selected_files.clone(), None),
            SelectionType::Dir => (convert.selected_dirs.clone(), None),
            SelectionType::Tree => {
                (self.convert.tree.checked_files(), Some(convert.tree_roots.clone()))
            }
        };

        if inputs.is_empty() {
            self.notify.warn(self.i18n.t(I18nKey::ConvertNoInput));
            return;
        }

        self.convert.task.start(
            &self.async_rt,
            ctx,
            inputs,
            convert.output.clone(),
            convert.format,
            roots,
        );
    }
}
