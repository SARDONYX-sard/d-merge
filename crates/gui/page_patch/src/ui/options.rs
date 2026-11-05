//! The panels above the mod list: execution mode, runtime, parser, options and the directories.

use egui::Label;
use egui_widgets::shadcn::{Button, Checkbox, ComboBox, Icon, RadioButton, TextEdit};
use gui_shared::{
    fs::{find_existing_dir_or_ancestor, open_existing_dir_or_ancestor},
    i18n::I18nKey,
    settings::{DataMode, pages::patch::behavior::ParserMode},
};

use super::PatchView;

impl PatchView<'_> {
    /// Renders the execution-mode radio buttons and global option checkboxes.
    ///
    /// Contains: VFS / Manual mode, target runtime combo, debug output,
    /// auto-remove meshes, generate FNIS ESP, auto-run, transparent.
    pub(super) fn ui_top_options(&mut self, ui: &mut egui::Ui) {
        let panel = egui::Panel::top("ui_top_options").frame(self.ctx.panel_frame);

        panel.show(ui, |ui| {
            ui.horizontal(|ui| {
                self.ui_execution_mode(ui);

                ui.separator();

                self.ui_target_runtime_box(ui);
                self.ui_parser_mode(ui);
                self.ui_behavior_options(ui);
            });

            ui.add_space(6.0);
        });
    }

    fn ui_execution_mode(&mut self, ui: &mut egui::Ui) {
        ui.add(Label::new(self.ctx.i18n.t(I18nKey::ExecutionModeLabel)));

        let is_fetching = self.page.fetch.is_fetching();
        ui.add_enabled_ui(!is_fetching, |ui| {
            let items = [
                (DataMode::Vfs, self.ctx.i18n.t(I18nKey::VfsMode), self.ctx.i18n.t(I18nKey::VfsModeHover)),
                (DataMode::Manual, self.ctx.i18n.t(I18nKey::ManualMode), self.ctx.i18n.t(I18nKey::ManualModeHover)),
            ];

            for (mode, label, hover) in items {
                if ui
                    .add(RadioButton::new(self.ctx.settings.behavior.mode == mode, label))
                    .on_hover_text(hover)
                    .clicked()
                {
                    self.ctx.settings.behavior.mode = mode;
                    self.page.fetch_mods(self.ctx.settings);
                }
            }
        });
    }

    /// Renders the target Skyrim runtime combo box (LE / SE / VR).
    ///
    /// In VFS mode, changing the runtime triggers a registry-based
    /// auto-detect of the data directory (Windows only).
    fn ui_target_runtime_box(&mut self, ui: &mut egui::Ui) {
        use skyrim_data_dir::Runtime;

        ui.horizontal_top(|ui| {
            ui.add(Label::new(self.ctx.i18n.t(I18nKey::RuntimeTargetLabel)))
                .on_hover_text(self.ctx.i18n.t(I18nKey::RuntimeTargetHover));

            const RUNTIMES: [(Runtime, &str); Runtime::ALL.len()] = {
                const fn make_runtimes() -> [(Runtime, &'static str); Runtime::ALL.len()] {
                    let mut runtimes = [(Runtime::Le, ""); Runtime::ALL.len()];
                    let mut i = 0;

                    while i < Runtime::ALL.len() {
                        let runtime = Runtime::ALL[i];
                        runtimes[i] = (runtime, runtime.as_str());
                        i += 1;
                    }

                    runtimes
                }

                make_runtimes()
            };

            let runtime = &mut self.ctx.settings.behavior.target_runtime;
            let mut index = RUNTIMES.iter().position(|(r, _)| r == runtime).unwrap_or(0);
            if ComboBox::from_id_salt("target_runtime")
                .width(100.0)
                .show_index(ui, &mut index, RUNTIMES.len(), |i| RUNTIMES[i].1)
                .changed()
            {
                *runtime = RUNTIMES[index].0;

                if self.ctx.settings.behavior.mode == DataMode::Vfs {
                    #[cfg(target_os = "windows")]
                    self.page.detect_vfs_data_dir(self.ctx.settings, self.ctx.notify, self.ctx.i18n);
                }
            }
        });
    }

    fn ui_parser_mode(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_top(|ui| {
            ui.label(self.ctx.i18n.t(I18nKey::ParserModeLabel))
                .on_hover_text(self.ctx.i18n.t(I18nKey::ParserModeHover));

            let modes = [
                (ParserMode::Strict, self.ctx.i18n.t(I18nKey::ParserModeStrict)),
                (ParserMode::Lenient, self.ctx.i18n.t(I18nKey::ParserModeLenient)),
            ];
            let parser_mode = &mut self.ctx.settings.behavior.parser_mode;
            let mut index = modes.iter().position(|(m, _)| m == parser_mode).unwrap_or(0);
            if ComboBox::from_id_salt("parser_mode")
                .width(100.0)
                .show_index(ui, &mut index, modes.len(), |i| modes[i].1)
                .changed()
            {
                *parser_mode = modes[index].0;
            }
        });
    }

    /// debug output, auto-remove meshes, generate FNIS ESP, auto-run,
    fn ui_behavior_options(&mut self, ui: &mut egui::Ui) {
        for (value, label, hover) in [
            (&mut self.ctx.settings.behavior.enable_debug_output, I18nKey::DebugOutput, I18nKey::DebugOutputHover),
            (
                &mut self.ctx.settings.behavior.auto_remove_meshes,
                I18nKey::AutoRemoveMeshes,
                I18nKey::AutoRemoveMeshesHover,
            ),
            (
                &mut self.ctx.settings.behavior.generate_fnis_esp,
                I18nKey::GenerateFnisEspLabel,
                I18nKey::GenerateFnisEspHover,
            ),
            (&mut self.ctx.settings.behavior.skeleton_arm_fix, I18nKey::ArmFixLabel, I18nKey::ArmFixHover),
            (&mut self.ctx.settings.behavior.auto_run, I18nKey::AutoRun, I18nKey::AutoRunHover),
        ] {
            ui.add(Checkbox::new(value, self.ctx.i18n.t(label))).on_hover_text(self.ctx.i18n.t(hover));
        }
    }

    /// UI
    /// ```txt
    /// |<---label--->|<------ stretch ------>|<-60->|<-60->|
    /// | Skyrim      | path..................| Auto | ...  |
    /// | Output      | path..................|      | ...  |
    /// ```
    pub(super) fn ui_paths(&mut self, ui: &mut egui::Ui) {
        let panel = egui::Panel::top("top_paths").frame(self.ctx.panel_frame);

        panel.show(ui, |ui| {
            self.ui_skyrim_dir_row(ui);
            ui.add_space(3.0);
            self.ui_output_dir_row(ui);
            ui.add_space(8.0);
        });
    }

    /// UI
    /// ```txt
    /// |<---label--->|<------ stretch ------>|<-60->|<-60->|
    /// | Skyrim      | path..................| Auto | ...  |
    /// ```
    fn ui_skyrim_dir_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_sized([LABEL_WIDTH, BUTTON_HEIGHT], Button::new(self.ctx.i18n.t(I18nKey::SkyrimDataDirLabel)))
                .on_hover_text(self.ctx.i18n.t(I18nKey::OpenSelectedPathHover))
                .clicked()
                && let Err(err) = open_existing_dir_or_ancestor(self.ctx.settings.current_skyrim_data_dir())
            {
                self.ctx.notify.error(err);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(RIGHT_MARGIN);

                if ui
                    .add_sized(
                        [BUTTON_WIDTH, BUTTON_HEIGHT],
                        Button::new(self.ctx.i18n.t(I18nKey::SelectButton)).icon(Icon::FolderSearch).full_width(),
                    )
                    .clicked()
                {
                    let dialog = match find_existing_dir_or_ancestor(self.ctx.settings.current_skyrim_data_dir()) {
                        Ok(abs_path) => gui_shared::rfd::FileDialog::new().set_directory(abs_path),
                        Err(_) => gui_shared::rfd::FileDialog::new(),
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        *self.ctx.settings.current_skyrim_data_dir_mut() = dir.display().to_string();
                        self.page.fetch_mods(self.ctx.settings);
                    }
                }

                #[cfg(target_os = "windows")]
                {
                    if self.ctx.settings.behavior.mode == DataMode::Vfs {
                        if ui
                            .add_sized(
                                [BUTTON_WIDTH, BUTTON_HEIGHT],
                                Button::new(self.ctx.i18n.t(I18nKey::AutoDetectButton))
                                    .icon(Icon::ScanSearch)
                                    .full_width(),
                            )
                            .on_hover_text(self.ctx.i18n.t(I18nKey::AutoDetectHover))
                            .clicked()
                        {
                            self.page.detect_vfs_data_dir(self.ctx.settings, self.ctx.notify, self.ctx.i18n);
                        }
                    } else {
                        ui.allocate_ui(egui::vec2(BUTTON_WIDTH, ROW_HEIGHT), |_| {});
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    ui.allocate_ui(egui::vec2(BUTTON_WIDTH, ROW_HEIGHT), |_| {});
                }

                let response = match self.ctx.settings.behavior.mode {
                    DataMode::Vfs => {
                        if self.page.first_frame && self.ctx.settings.vfs.skyrim_data_dir.trim().is_empty() {
                            self.page.detect_vfs_data_dir(self.ctx.settings, self.ctx.notify, self.ctx.i18n);
                            return;
                        }

                        path_text_edit(ui, &mut self.ctx.settings.vfs.skyrim_data_dir, None)
                    }

                    DataMode::Manual => path_text_edit(
                        ui,
                        &mut self.ctx.settings.manual.skyrim_data_dir,
                        Some("D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*"),
                    ),
                };

                if self.page.first_frame || response.changed() {
                    self.page.fetch_mods(self.ctx.settings);
                }
            });
        });
    }

    /// Renders the output-directory row (label button + text field + folder picker).
    ///
    /// UI
    /// ```txt
    /// |<---label--->|<------ stretch ------>|<-60->|<-60->|
    /// | Output      | path..................|      | ...  |
    /// ```
    fn ui_output_dir_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_sized([LABEL_WIDTH, BUTTON_HEIGHT], Button::new(self.ctx.i18n.t(I18nKey::OutputDirLabel)))
                .on_hover_text(self.ctx.i18n.t(I18nKey::OpenSelectedPathHover))
                .clicked()
                && let Err(err) =
                    open_existing_dir_or_ancestor(std::path::Path::new(self.ctx.settings.current_output_dir()))
            {
                self.ctx.notify.error(err);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(RIGHT_MARGIN);

                if ui
                    .add_sized(
                        [BUTTON_WIDTH, BUTTON_HEIGHT],
                        Button::new(self.ctx.i18n.t(I18nKey::SelectButton)).icon(Icon::FolderSearch).full_width(),
                    )
                    .clicked()
                {
                    let dialog = if self.ctx.settings.current_output_dir().is_empty() {
                        gui_shared::rfd::FileDialog::new()
                    } else {
                        match find_existing_dir_or_ancestor(self.ctx.settings.current_output_dir()) {
                            Ok(abs_path) => gui_shared::rfd::FileDialog::new().set_directory(abs_path),
                            Err(err) => {
                                self.ctx.notify.error(format!("Couldn't find output dir or ancestor: {err}"));
                                return;
                            }
                        }
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        *self.ctx.settings.current_output_dir_mut() = dir.display().to_string();
                    }
                }

                ui.allocate_ui(egui::vec2(BUTTON_WIDTH, ROW_HEIGHT), |_| {});

                path_text_edit(ui, self.ctx.settings.current_output_dir_mut(), None);
            });
        });
    }
}

const LABEL_WIDTH: f32 = 140.0;
const BUTTON_WIDTH: f32 = 90.0;
const ROW_HEIGHT: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 32.0;
const RIGHT_MARGIN: f32 = 8.0;

fn path_text_edit(ui: &mut egui::Ui, value: &mut String, hint: Option<&str>) -> egui::Response {
    let mut text_edit = TextEdit::singleline(value).desired_width(ui.available_width());
    if let Some(hint) = hint {
        text_edit = text_edit.hint_text(hint);
    }
    ui.add(text_edit)
}
