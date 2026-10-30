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
//! # Ownership
//! - persisted inputs: `settings.convert`, borrowed through [`ConvertPageCtx`]
//! - transient state (loaded tree, progress): [`ConvertPage`], owned by the app
//!
//! The page never touches the rest of the app; everything it needs comes in [`ConvertPageCtx`].

mod task;
mod tree;

use egui_widgets::{
    notify::Notification,
    shadcn::{Button, ComboBox, RadioButton, TextEdit},
};
use gui_shared::{
    fs::open_existing_dir_or_ancestor,
    i18n::{I18nKey, I18nMap},
    settings::convert::{ConvertSettings, OutFormat, SelectionType},
};

use self::{task::ConvertTask, tree::TreeView};

/// Extensions the converter accepts as input.
const INPUT_EXTENSIONS: [&str; 4] = ["hkx", "xml", "json", "yaml"];

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct ConvertPageCtx<'a> {
    /// Persisted page settings (`settings.convert`).
    pub settings: &'a mut ConvertSettings,
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// Frame (background) of the page's panels, from the app theme.
    pub panel_frame: egui::Frame,
    /// Runtime the conversion runs on.
    pub runtime: &'a tokio::runtime::Runtime,
}

/// Transient Convert page state (see the crate docs). Keep one in the app.
#[derive(Default)]
pub struct ConvertPage {
    tree: TreeView,
    task: ConvertTask,
}

impl std::fmt::Debug for ConvertPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConvertPage")
            .field("running", &self.task.is_running())
            .finish_non_exhaustive()
    }
}

impl ConvertPage {
    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, mut ctx: ConvertPageCtx<'_>) {
        // NOTE: Bottom panel first, then the central area (egui panel order).
        egui::Panel::bottom("convert_actions")
            .frame(ctx.panel_frame)
            .show(ui, |ui| self.ui_convert_button(ui, &mut ctx));

        egui::CentralPanel::default().frame(ctx.panel_frame).show(ui, |ui| {
            let all_clear = Button::new(ctx.i18n.t(I18nKey::ConvertAllClearButton)).full_width();
            if ui.add(all_clear).clicked() {
                self.clear_all(ctx.settings);
            }
            ui.add_space(8.0);

            egui::Grid::new("convert_form").num_columns(2).spacing([8.0, 8.0]).show(ui, |ui| {
                output_row(ui, &mut ctx);
                self.selection_row(ui, &mut ctx);
                format_row(ui, &mut ctx);
            });

            ui.separator();
            self.inputs(ui, &mut ctx);
        });
    }

    /// Call every frame (even when the page is hidden): reports a finished conversion.
    pub fn poll(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        match self.task.take_finished() {
            Some(Ok(())) => notify.success(i18n.t(I18nKey::ConvertDone)),
            Some(Err(err)) => {
                tracing::error!("{err}");
                notify.error(err);
            }
            None => {}
        }
    }

    // ── Form rows ────────────────────────────────────────────────────────────

    /// `Selection type: (•) Directories ( ) Tree ( ) Files [Select]`
    fn selection_row(&self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        ui.label(ctx.i18n.t(I18nKey::ConvertSelectionTypeLabel));

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
                let current = &mut ctx.settings.selection_type;
                if ui
                    .add(RadioButton::new(*current == kind, ctx.i18n.t(label)))
                    .on_hover_text(ctx.i18n.t(hover))
                    .clicked()
                    && *current != kind
                {
                    *current = kind;
                    // Statuses are keyed by path; clear them so modes do not mix.
                    self.task.clear_statuses();
                }
            }

            ui.add_space(8.0);
            if ui.add(Button::new(ctx.i18n.t(I18nKey::SelectButton))).clicked() {
                self.pick_inputs(ctx.settings);
            }
        });
        ui.end_row();
    }

    // ── Inputs ───────────────────────────────────────────────────────────────

    /// Tree (Tree mode) or a removable list of paths (Files / Directories mode).
    fn inputs(&mut self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        if ctx.settings.selection_type == SelectionType::Tree {
            if let Err(err) = self.tree.ensure_loaded(&ctx.settings.tree_roots) {
                ctx.notify.error(err);
            }

            ui.horizontal(|ui| {
                let tree = &mut self.tree;
                let select_label = if tree.has_checked() {
                    I18nKey::ConvertUnselectAll
                } else {
                    I18nKey::ConvertSelectAll
                };
                if ui.add(Button::new(ctx.i18n.t(select_label))).clicked() {
                    tree.toggle_select_all();
                }

                let expand_label = if tree.is_any_expanded() {
                    I18nKey::ConvertCollapseAll
                } else {
                    I18nKey::ConvertExpandAll
                };
                if ui.add(Button::new(ctx.i18n.t(expand_label))).clicked() {
                    tree.toggle_expand_all();
                }
            });

            let task = &self.task;
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                self.tree.ui(ui, &|path| task.status_of(path));
            });
            return;
        }

        // Files / Directories: `(status) path [✖]`
        let paths = match ctx.settings.selection_type {
            SelectionType::Dir => &mut ctx.settings.selected_dirs,
            SelectionType::Files | SelectionType::Tree => &mut ctx.settings.selected_files,
        };
        let task = &self.task;
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
    fn pick_inputs(&self, settings: &mut ConvertSettings) {
        let start_dir = match settings.selection_type {
            SelectionType::Files => settings.selected_files.first(),
            SelectionType::Dir => settings.selected_dirs.first(),
            SelectionType::Tree => settings.tree_roots.first(),
        }
        .cloned()
        .unwrap_or_default();
        let dialog = rfd::FileDialog::new().set_directory(&start_dir);

        let to_strings = |paths: Vec<std::path::PathBuf>| -> Vec<String> {
            paths.iter().map(|p| p.display().to_string()).collect()
        };

        let picked = match settings.selection_type {
            SelectionType::Files => dialog
                .add_filter("Havok", &INPUT_EXTENSIONS)
                .pick_files()
                .map(|files| (&mut settings.selected_files, to_strings(files))),
            SelectionType::Dir => {
                dialog.pick_folders().map(|dirs| (&mut settings.selected_dirs, to_strings(dirs)))
            }
            SelectionType::Tree => {
                dialog.pick_folders().map(|dirs| (&mut settings.tree_roots, to_strings(dirs)))
            }
        };

        if let Some((target, paths)) = picked {
            *target = paths;
            self.task.clear_statuses();
        }
    }

    /// "All Clear": forget the inputs of the current mode, the output and the statuses.
    fn clear_all(&mut self, settings: &mut ConvertSettings) {
        match settings.selection_type {
            SelectionType::Files => settings.selected_files.clear(),
            SelectionType::Dir => settings.selected_dirs.clear(),
            SelectionType::Tree => {
                settings.tree_roots.clear();
                self.tree.clear();
            }
        }
        settings.output.clear();
        self.task.clear_statuses();
    }

    // ── Run ──────────────────────────────────────────────────────────────────

    fn ui_convert_button(&mut self, ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
        let running = self.task.is_running();
        let label =
            ctx.i18n.t(if running { I18nKey::ConvertingButton } else { I18nKey::ConvertButton });

        ui.add_space(4.0);
        let button = Button::new(label).primary().full_width().min_size(egui::vec2(0.0, 40.0));
        if ui.add_enabled(!running, button).clicked() {
            self.start_convert(ui.ctx(), ctx);
        }
        ui.add_space(4.0);
    }

    fn start_convert(&mut self, egui_ctx: &egui::Context, ctx: &mut ConvertPageCtx<'_>) {
        let settings = &*ctx.settings;
        let (inputs, roots) = match settings.selection_type {
            SelectionType::Files => (settings.selected_files.clone(), None),
            SelectionType::Dir => (settings.selected_dirs.clone(), None),
            SelectionType::Tree => (self.tree.checked_files(), Some(settings.tree_roots.clone())),
        };

        if inputs.is_empty() {
            ctx.notify.warn(ctx.i18n.t(I18nKey::ConvertNoInput));
            return;
        }

        self.task.start(
            ctx.runtime,
            egui_ctx,
            inputs,
            settings.output.clone(),
            settings.format,
            roots,
        );
    }
}

/// `Output: [path] [Select]` (the label button opens the directory).
fn output_row(ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
    let output = &mut ctx.settings.output;

    if ui
        .add(Button::new(ctx.i18n.t(I18nKey::ConvertOutputLabel)))
        .on_hover_text(ctx.i18n.t(I18nKey::ConvertOutputHover))
        .clicked()
        && let Err(err) = open_existing_dir_or_ancestor(std::path::Path::new(output.as_str()))
    {
        tracing::error!(err);
    }

    // Right-to-left: the button takes its width first, the text edit gets the rest.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        if ui.add(Button::new(ctx.i18n.t(I18nKey::SelectButton))).clicked()
            && let Some(dir) = rfd::FileDialog::new().set_directory(output.as_str()).pick_folder()
        {
            *output = dir.display().to_string();
        }
        let width = ui.available_width();
        ui.add(TextEdit::singleline(output).desired_width(width));
    });
    ui.end_row();
}

/// `Output format: [Amd64 ▼]`
fn format_row(ui: &mut egui::Ui, ctx: &mut ConvertPageCtx<'_>) {
    ui.label(ctx.i18n.t(I18nKey::ConvertOutputFormatLabel));

    let format = &mut ctx.settings.format;
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
