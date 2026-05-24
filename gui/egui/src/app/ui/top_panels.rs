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

use egui::Separator;

use crate::{
    app::{App, state::FetchState},
    i18n::I18nKey,
    settings::{DataMode, ui::Theme},
};

impl App {
    /// Renders the execution-mode radio buttons and global option checkboxes.
    ///
    /// Contains: VFS / Manual mode, target runtime combo, debug output,
    /// auto-remove meshes, generate FNIS ESP, transparent, auto-run, theme.
    pub(crate) fn ui_execution_mode(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_execution_mode");
        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.t(I18nKey::ExecutionModeLabel));
                let items = [
                    (
                        DataMode::Vfs,
                        self.t(I18nKey::VfsMode).to_string(),
                        self.t(I18nKey::VfsModeHover).to_string(),
                    ),
                    (
                        DataMode::Manual,
                        self.t(I18nKey::ManualMode).to_string(),
                        self.t(I18nKey::ManualModeHover).to_string(),
                    ),
                ];
                let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);

                ui.add_enabled_ui(!is_fetching, |ui| {
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

                ui.add(Separator::default().vertical());

                self.ui_target_runtime_box(ui);

                let debug_output_label = self.t(I18nKey::DebugOutput).to_string();
                let debug_output_hover = self.t(I18nKey::DebugOutputHover).to_string();
                ui.checkbox(&mut self.settings.behavior.enable_debug_output, debug_output_label)
                    .on_hover_text(debug_output_hover);

                let auto_remove_meshes_label = self.t(I18nKey::AutoRemoveMeshes).to_string();
                let auto_remove_meshes_hover = self.t(I18nKey::AutoRemoveMeshesHover).to_string();
                ui.checkbox(
                    &mut self.settings.behavior.auto_remove_meshes,
                    auto_remove_meshes_label,
                )
                .on_hover_text(auto_remove_meshes_hover);

                let generate_fnis_esp_label = self.t(I18nKey::GenerateFnisEspLabel).to_string();
                let generate_fnis_esp_hover = self.t(I18nKey::GenerateFnisEspHover).to_string();
                ui.checkbox(&mut self.settings.behavior.generate_fnis_esp, generate_fnis_esp_label)
                    .on_hover_text(generate_fnis_esp_hover);

                ui.add_space(60.0);

                let auto_run_label = self.t(I18nKey::AutoRun).to_string();
                let auto_run_hover = self.t(I18nKey::AutoRunHover).to_string();
                ui.checkbox(&mut self.settings.behavior.auto_run, auto_run_label)
                    .on_hover_text(auto_run_hover);

                let transparent_label = self.t(I18nKey::Transparent).to_string();
                let transparent_hover = self.t(I18nKey::TransparentHover).to_string();
                ui.checkbox(&mut self.settings.ui.transparent, transparent_label)
                    .on_hover_text(transparent_hover);

                ui.separator();
                self.ui_theme_box(ui);
            });

            ui.add_space(8.0);
        });
    }

    /// Renders the theme combo box (System / Dark / Light).
    fn ui_theme_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.t(I18nKey::ThemeLabel)).on_hover_text(self.t(I18nKey::ThemeHover));

            egui::ComboBox::from_id_salt("theme")
                .selected_text(self.settings.ui.theme.as_str())
                .show_ui(ui, |ui| {
                    for theme in [Theme::System, Theme::Dark, Theme::Light] {
                        if ui
                            .selectable_value(&mut self.settings.ui.theme, theme, theme.as_str())
                            .changed()
                        {
                            ui.ctx().set_theme(self.settings.ui.theme);
                        }
                    }
                });
        });
    }

    /// Renders the target Skyrim runtime combo box (LE / SE / VR).
    ///
    /// In VFS mode, changing the runtime triggers a registry-based
    /// auto-detect of the data directory (Windows only).
    fn ui_target_runtime_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(self.t(I18nKey::RuntimeTargetLabel))
                .on_hover_text(self.t(I18nKey::RuntimeTargetHover));

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

    /// Renders the Skyrim data-directory row (label button + text field +
    /// optional auto-detect + folder picker).
    pub(crate) fn ui_skyrim_dir(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::top("top_data_dir");
        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(self.t(I18nKey::SkyrimDataDirLabel)).clicked()
                    && let Err(err) = crate::app::dir_utils::open_existing_dir_or_ancestor(
                        self.settings.current_skyrim_data_dir(),
                    )
                {
                    self.notify_error(err);
                }

                self.draw_skyrim_dir_ui(ui);

                #[cfg(target_os = "windows")]
                if self.settings.behavior.mode == DataMode::Vfs
                    && ui
                        .add_sized(
                            [60.0, 40.0],
                            egui::Button::new(self.t(I18nKey::AutoDetectButton)),
                        )
                        .on_hover_text(self.t(I18nKey::AutoDetectHover))
                        .clicked()
                {
                    self.update_vfs_skyrim_data_dir_by_reg();
                }

                if ui
                    .add_sized([60.0, 40.0], egui::Button::new(self.t(I18nKey::SelectButton)))
                    .clicked()
                {
                    let dialog = match crate::app::dir_utils::find_existing_dir_or_ancestor(
                        self.settings.current_skyrim_data_dir(),
                    ) {
                        Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                        Err(_) => rfd::FileDialog::new(),
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        match self.settings.behavior.mode {
                            DataMode::Vfs => {
                                self.settings.vfs.skyrim_data_dir = dir.display().to_string();
                                self.update_mod_list();
                            }
                            DataMode::Manual => {
                                self.settings.manual.skyrim_data_dir = dir.display().to_string();
                                self.update_mod_list();
                            }
                        }
                    }
                }
            });
        });
    }

    /// Renders the output-directory row (label button + text field + folder picker).
    pub(crate) fn ui_output_dir(&mut self, ctx: &egui::Context) {
        let use_transparent = self.settings.ui.transparent;
        let mut panel = egui::TopBottomPanel::top("top_output_dir");
        if use_transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(self.t(I18nKey::OutputDirLabel)).clicked()
                    && let Err(err) = crate::app::dir_utils::open_existing_dir_or_ancestor(
                        std::path::Path::new(self.settings.current_output_dir()),
                    )
                {
                    self.notify_error(err);
                }

                let text_line = egui::TextEdit::singleline(self.settings.current_output_dir_mut());
                let text_line = if use_transparent {
                    text_line.background_color(egui::Color32::TRANSPARENT)
                } else {
                    text_line
                };
                ui.add_sized([ui.available_width() * 0.9, 40.0], text_line);

                if ui
                    .add_sized([60.0, 40.0], egui::Button::new(self.t(I18nKey::SelectButton)))
                    .clicked()
                {
                    let dialog = if !self.settings.current_output_dir().is_empty() {
                        match crate::app::dir_utils::find_existing_dir_or_ancestor(
                            self.settings.current_output_dir(),
                        ) {
                            Ok(abs_path) => rfd::FileDialog::new().set_directory(abs_path),
                            Err(err) => {
                                self.notify_error(format!(
                                    "Couldn't find output dir or ancestor: {err}"
                                ));
                                return;
                            }
                        }
                    } else {
                        rfd::FileDialog::new()
                    };

                    if let Some(dir) = dialog.pick_folder() {
                        let new_output_dir = dir.display().to_string();
                        let old_output_dir = self.settings.current_output_dir_mut();
                        if new_output_dir != *old_output_dir {
                            *old_output_dir = new_output_dir;
                        }
                    }
                }
            });
        });
    }
}
