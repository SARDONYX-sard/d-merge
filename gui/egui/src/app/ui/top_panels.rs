//! Top-panel UI: execution mode, directory pickers, and search bar.

use d_merge_gui_shared::{
    fetch::FetchState,
    fs::{find_existing_dir_or_ancestor, open_existing_dir_or_ancestor},
    i18n::I18nKey,
    settings::{
        DataMode,
        ui::theme::{CustomTheme, Theme},
    },
};
use egui::Label;

use crate::{
    app::App,
    theme::{set_theme, themed_top_bottom_panel},
    ui::shadcn_compat::{button, button_with_icon, checkbox, enum_select, radio_value},
};

impl App {
    /// Renders the execution-mode radio buttons and global option checkboxes.
    ///
    /// Contains: VFS / Manual mode, target runtime combo, debug output,
    /// auto-remove meshes, generate FNIS ESP, auto-run, transparent, theme.
    pub(crate) fn ui_top_options(&mut self, ctx: &egui::Context) {
        let panel = themed_top_bottom_panel(
            egui::TopBottomPanel::top("ui_top_options"),
            self.settings.ui.theme,
            self.theme_manager.current_bg_color(),
        );

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_execution_mode(ui);

                ui.separator();

                self.ui_target_runtime_box(ui);
                self.ui_behavior_options(ui);

                ui.add_space(60.0);
                ui.separator();

                self.ui_theme_box(ui);

                self.ui_bg_color_picker(ui);
            });
        });
    }

    fn ui_execution_mode(&mut self, ui: &mut egui::Ui) {
        ui.add(Label::new(self.i18n.t(I18nKey::ExecutionModeLabel)));

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
                if radio_value(ui, &mut self.settings.behavior.mode, mode, label)
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
            ui.add(Label::new(self.i18n.t(I18nKey::RuntimeTargetLabel)))
                .on_hover_text(self.i18n.t(I18nKey::RuntimeTargetHover));

            const RUNTIMES: [(skyrim_data_dir::Runtime, &str); 3] = [
                (skyrim_data_dir::Runtime::Le, skyrim_data_dir::Runtime::Le.as_str()),
                (skyrim_data_dir::Runtime::Se, skyrim_data_dir::Runtime::Se.as_str()),
                (skyrim_data_dir::Runtime::Vr, skyrim_data_dir::Runtime::Vr.as_str()),
            ];
            if enum_select(
                ui,
                &mut self.settings.behavior.target_runtime,
                &RUNTIMES,
                Some([100.0, 30.0]),
            )
            .changed()
                && self.settings.behavior.mode == DataMode::Vfs
            {
                #[cfg(target_os = "windows")]
                self.update_vfs_skyrim_data_dir_by_reg();
            }
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
            ui.add(Label::new(self.i18n.t(I18nKey::ThemeLabel)))
                .on_hover_text(self.i18n.t(I18nKey::ThemeHover));

            let themes: [(Theme, &str); 4] = [
                (Theme::System, self.i18n.t(I18nKey::ThemeSelectSystem)),
                (Theme::Dark, self.i18n.t(I18nKey::ThemeSelectDark)),
                (Theme::Light, self.i18n.t(I18nKey::ThemeSelectLight)),
                (Theme::Custom, self.i18n.t(I18nKey::ThemeSelectCustom)),
            ];

            if enum_select(ui, &mut self.settings.ui.theme, &themes, Some([120.0, 30.0])).changed()
            {
                set_theme(ui.ctx(), self.settings.ui.theme, self.theme_manager.editing.as_ref());
            }
        });
    }

    fn ui_bg_color_picker(&mut self, ui: &mut egui::Ui) {
        if matches!(self.settings.ui.theme, Theme::Custom) {
            fn toggle_value(
                ui: &mut egui::Ui,
                pressed: &mut bool,
                text: impl Into<egui::WidgetText>,
            ) -> egui::Response {
                egui_shadcn::Toggle::new(pressed, text).show(ui)
            }
            toggle_value(ui, &mut self.show_theme_editor, self.i18n.t(I18nKey::ThemeEditorButton))
                .on_hover_text(self.i18n.t(I18nKey::ThemeEditorButtonHover));
        }

        if matches!(self.settings.ui.theme, Theme::Custom)
            && self.show_theme_editor
            && let Some(update) = self.theme_manager.show(ui.ctx(), &mut self.show_theme_editor)
        {
            // Persist only the name.
            self.settings.ui.custom_theme = CustomTheme {
                selected_theme: Some(update.selected_name.clone()),
                themes_dir: self.theme_manager.cache.dir.to_string_lossy().to_string(),
            };

            // Live-apply whenever colors changed or a new preset was loaded.
            if let Some(preset) = &update.preset {
                crate::ui::theme::apply(preset, ui.ctx());
                set_theme(ui.ctx(), self.settings.ui.theme, Some(preset));
            }
        }
    }

    /// UI
    /// ```txt
    /// |<---label--->|<------ stretch ------>|<-60->|<-60->|
    /// | Skyrim      | path..................| Auto | ...  |
    /// | Output      | path..................|      | ...  |
    /// ```
    pub(crate) fn ui_paths(&mut self, ctx: &egui::Context) {
        let panel = themed_top_bottom_panel(
            egui::TopBottomPanel::top("top_paths"),
            self.settings.ui.theme,
            self.theme_manager.current_bg_color(),
        );

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
                    button(self.i18n.t(I18nKey::SkyrimDataDirLabel)),
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
                        button_with_icon(
                            self.i18n.t(I18nKey::SelectButton),
                            egui_shadcn::LucideIcon::FolderSearch,
                        ),
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
                                button_with_icon(
                                    self.i18n.t(I18nKey::AutoDetectButton),
                                    egui_shadcn::LucideIcon::ScanSearch,
                                ),
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

                        path_text_edit(ui, &mut self.settings.vfs.skyrim_data_dir, None)
                    }

                    DataMode::Manual => path_text_edit(
                        ui,
                        &mut self.settings.manual.skyrim_data_dir,
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
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    [LABEL_WIDTH, BUTTON_HEIGHT],
                    button(self.i18n.t(I18nKey::OutputDirLabel)),
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
                        button_with_icon(
                            self.i18n.t(I18nKey::SelectButton),
                            egui_shadcn::LucideIcon::FolderSearch,
                        ),
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

                path_text_edit(ui, self.settings.current_output_dir_mut(), None);
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
    let mut input = egui_shadcn::Input::new(value).desired_width(ui.available_width());

    if let Some(hint) = hint {
        input = input.placeholder(hint);
    }

    input.show(ui)
}
