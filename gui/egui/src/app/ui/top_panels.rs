//! Top-panel UI: execution mode, directory pickers, and search bar.
//!
//! All four panels are [`egui::TopBottomPanel::top`] and must be registered
//! before the [`egui::CentralPanel`] in [`eframe::App::update`].
//!
//! # Transparent mode
//! When [`AppSettings::transparent`] is `true` every panel is built with
//! [`egui::Frame::new()`] (no background fill) so the OS window chrome shows
//! through.  The pattern `if transparent { panel.frame(Frame::new()) }` is
//! repeated per panel because [`egui::TopBottomPanel`] consumes `self` on
//! each builder call and cannot be stored across the branch.

use d_merge_gui_shared::{
    fetch::FetchState,
    fs::{find_existing_dir_or_ancestor, open_existing_dir_or_ancestor},
    i18n::I18nKey,
    settings::{DataMode, ui::Theme},
};

use crate::app::App;

impl App {
    /// Renders the execution-mode radio buttons and global option checkboxes.
    ///
    /// Contains: VFS / Manual mode, target runtime combo, debug output,
    /// auto-remove meshes, generate FNIS ESP, auto-run, transparent, theme.
    pub(crate) fn ui_top_options(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_execution_mode");
        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_execution_mode(ui);

                ui.separator();

                self.ui_target_runtime_box(ui);
                self.ui_behavior_options(ui);

                ui.add_space(60.0);
                ui.separator();

                {
                    let (value, label, hover) = (
                        &mut self.settings.ui.transparent,
                        I18nKey::Transparent,
                        I18nKey::TransparentHover,
                    );
                    checkbox(ui, value, self.i18n.t(label)).on_hover_text(self.i18n.t(hover));
                }
                self.ui_theme_box(ui);
            });

            ui.add_space(8.0);
        });
    }

    fn ui_execution_mode(&mut self, ui: &mut egui::Ui) {
        ui.label(self.i18n.t(I18nKey::ExecutionModeLabel));

        let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);
        ui.add_enabled_ui(!is_fetching, |ui| {
            let items = [
                (DataMode::Vfs, self.i18n.t(I18nKey::VfsMode), self.i18n.t(I18nKey::VfsModeHover)),
                (
                    DataMode::Manual,
                    self.i18n.t(I18nKey::ManualMode),
                    self.i18n.t(I18nKey::ManualModeHover),
                ),
            ];

            for (mode, label, hover) in items {
                if ui
                    .radio_value(&mut self.settings.behavior.mode, mode, label)
                    .on_hover_text(hover)
                    .clicked()
                {
                    self.update_mod_list();
                }
            }
        });
    }

    /// Renders the target Skyrim runtime combo box (LE / SE / VR).
    ///
    /// In VFS mode, changing the runtime triggers a registry-based
    /// auto-detect of the data directory (Windows only).
    fn ui_target_runtime_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.i18n.t(I18nKey::RuntimeTargetLabel))
                .on_hover_text(self.i18n.t(I18nKey::RuntimeTargetHover));

            egui::ComboBox::from_id_salt("skyrim_runtime_target")
                .selected_text(self.settings.behavior.target_runtime.as_str())
                .show_ui(ui, |ui| {
                    let runtimes = [
                        (skyrim_data_dir::Runtime::Le, "SkyrimLE"),
                        (skyrim_data_dir::Runtime::Se, "SkyrimSE"),
                        (skyrim_data_dir::Runtime::Vr, "SkyrimVR"),
                    ];
                    for (runtime, label) in runtimes {
                        if ui
                            .selectable_value(
                                &mut self.settings.behavior.target_runtime,
                                runtime,
                                label,
                            )
                            .changed()
                            && self.settings.behavior.mode == DataMode::Vfs
                        {
                            #[cfg(target_os = "windows")]
                            self.update_vfs_skyrim_data_dir_by_reg();
                        }
                    }
                });
        });
    }

    /// debug output, auto-remove meshes, generate FNIS ESP, auto-run,
    fn ui_behavior_options(&mut self, ui: &mut egui::Ui) {
        for (value, label, hover) in [
            (
                &mut self.settings.behavior.enable_debug_output,
                I18nKey::DebugOutput,
                I18nKey::DebugOutputHover,
            ),
            (
                &mut self.settings.behavior.auto_remove_meshes,
                I18nKey::AutoRemoveMeshes,
                I18nKey::AutoRemoveMeshesHover,
            ),
            (
                &mut self.settings.behavior.generate_fnis_esp,
                I18nKey::GenerateFnisEspLabel,
                I18nKey::GenerateFnisEspHover,
            ),
            (&mut self.settings.behavior.auto_run, I18nKey::AutoRun, I18nKey::AutoRunHover),
        ] {
            checkbox(ui, value, self.i18n.t(label)).on_hover_text(self.i18n.t(hover));
        }
    }

    /// Renders the theme combo box (System / Dark / Light).
    fn ui_theme_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.i18n.t(I18nKey::ThemeLabel))
                .on_hover_text(self.i18n.t(I18nKey::ThemeHover));

            egui::ComboBox::from_id_salt("theme")
                .selected_text(self.settings.ui.theme.as_str())
                .show_ui(ui, |ui| {
                    for theme in [Theme::System, Theme::Dark, Theme::Light] {
                        if ui
                            .selectable_value(&mut self.settings.ui.theme, theme, theme.as_str())
                            .changed()
                        {
                            ui.ctx().set_theme(crate::to_egui_theme(self.settings.ui.theme));
                        }
                    }
                });
        });
    }

    /// UI
    /// ```txt
    /// |<---label--->|<------ stretch ------>|<-60->|<-60->|
    /// | Skyrim      | path..................| Auto | ...  |
    /// | Output      | path..................|      | ...  |
    /// ```
    pub(crate) fn ui_paths(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_paths");

        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            self.ui_skyrim_dir_row(ui);
            self.ui_output_dir_row(ui);
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
                .add_sized(
                    [LABEL_WIDTH, BUTTON_HEIGHT],
                    egui::Button::new(self.i18n.t(I18nKey::SkyrimDataDirLabel)),
                )
                .clicked()
                && let Err(err) =
                    open_existing_dir_or_ancestor(self.settings.current_skyrim_data_dir())
            {
                self.notify_error(err);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(RIGHT_MARGIN);

                if ui
                    .add_sized(
                        [BUTTON_WIDTH, BUTTON_HEIGHT],
                        egui::Button::new(self.i18n.t(I18nKey::SelectButton)),
                    )
                    .clicked()
                {
                    let dialog = match find_existing_dir_or_ancestor(
                        self.settings.current_skyrim_data_dir(),
                    ) {
                        Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                        Err(_) => rfd::FileDialog::new(),
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        *self.settings.current_skyrim_data_dir_mut() = dir.display().to_string();
                        self.update_mod_list();
                    }
                }

                #[cfg(target_os = "windows")]
                {
                    if self.settings.behavior.mode == DataMode::Vfs {
                        if ui
                            .add_sized(
                                [BUTTON_WIDTH, BUTTON_HEIGHT],
                                egui::Button::new(self.i18n.t(I18nKey::AutoDetectButton)),
                            )
                            .on_hover_text(self.i18n.t(I18nKey::AutoDetectHover))
                            .clicked()
                        {
                            self.update_vfs_skyrim_data_dir_by_reg();
                        }
                    } else {
                        ui.allocate_ui(egui::vec2(BUTTON_WIDTH, ROW_HEIGHT), |_| {});
                    }
                }

                #[cfg(not(target_os = "windows"))]
                {
                    ui.allocate_ui(egui::vec2(BUTTON_WIDTH, ROW_HEIGHT), |_| {});
                }

                let response = match self.settings.behavior.mode {
                    DataMode::Vfs => {
                        if self.is_first_render
                            && self.settings.vfs.skyrim_data_dir.trim().is_empty()
                        {
                            self.update_vfs_skyrim_data_dir_by_reg();
                            return;
                        }

                        path_text_edit(
                            ui,
                            &mut self.settings.vfs.skyrim_data_dir,
                            self.settings.ui.transparent,
                            None,
                        )
                    }

                    DataMode::Manual => path_text_edit(
                        ui,
                        &mut self.settings.manual.skyrim_data_dir,
                        self.settings.ui.transparent,
                        Some("D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*"),
                    ),
                };

                if self.is_first_render || response.changed() {
                    self.update_mod_list();
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
        let transparent = self.settings.ui.transparent;

        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    [LABEL_WIDTH, BUTTON_HEIGHT],
                    egui::Button::new(self.i18n.t(I18nKey::OutputDirLabel)),
                )
                .clicked()
                && let Err(err) = open_existing_dir_or_ancestor(std::path::Path::new(
                    self.settings.current_output_dir(),
                ))
            {
                self.notify_error(err);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(RIGHT_MARGIN);

                if ui
                    .add_sized(
                        [BUTTON_WIDTH, BUTTON_HEIGHT],
                        egui::Button::new(self.i18n.t(I18nKey::SelectButton)),
                    )
                    .clicked()
                {
                    let dialog = if self.settings.current_output_dir().is_empty() {
                        rfd::FileDialog::new()
                    } else {
                        match find_existing_dir_or_ancestor(self.settings.current_output_dir()) {
                            Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                            Err(err) => {
                                self.notify_error(format!(
                                    "Couldn't find output dir or ancestor: {err}"
                                ));
                                return;
                            }
                        }
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        *self.settings.current_output_dir_mut() = dir.display().to_string();
                    }
                }

                ui.allocate_ui(egui::vec2(BUTTON_WIDTH, ROW_HEIGHT), |_| {});

                path_text_edit(ui, self.settings.current_output_dir_mut(), transparent, None);
            });
        });
    }
}

fn checkbox(
    ui: &mut egui::Ui,
    checked: &mut bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    ui.add(egui::Checkbox::new(checked, label))
}

const LABEL_WIDTH: f32 = 140.0;
const BUTTON_WIDTH: f32 = 70.0;
const ROW_HEIGHT: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 32.0;
const RIGHT_MARGIN: f32 = 8.0;

fn path_text_edit(
    ui: &mut egui::Ui,
    value: &mut String,
    transparent: bool,
    hint: Option<&str>,
) -> egui::Response {
    let mut edit = egui::TextEdit::singleline(value);

    if let Some(hint) = hint {
        edit = edit.hint_text(hint);
    }

    if transparent {
        edit = edit.background_color(egui::Color32::TRANSPARENT);
    }

    ui.add_sized([ui.available_width(), ROW_HEIGHT], edit)
}
