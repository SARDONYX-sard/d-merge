//! Top-panel UI: execution mode, directory pickers, and search bar.

use d_merge_gui_shared::{
    fetch::FetchState,
    fs::{find_existing_dir_or_ancestor, open_existing_dir_or_ancestor},
    i18n::I18nKey,
    settings::{DataMode, ui::Theme},
};
use egui::Label;

use crate::{
    app::App,
    set_theme,
    ui::shadcn_compat::{button, button_with_icon, enum_select, radio_value},
};

impl App {
    /// Renders the execution-mode radio buttons and global option checkboxes.
    ///
    /// Contains: VFS / Manual mode, target runtime combo, debug output,
    /// auto-remove meshes, generate FNIS ESP, auto-run, transparent, theme.
    pub(crate) fn ui_top_options(&mut self, ctx: &egui::Context) {
        let panel = super::themed_top_bottom_panel(
            egui::TopBottomPanel::top("ui_top_options"),
            self.settings.ui.theme,
            self.settings.ui.transparent,
        );

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_execution_mode(ui);

                ui.separator();

                self.ui_target_runtime_box(ui);
                self.ui_behavior_options(ui);

                ui.add_space(60.0);
                ui.separator();

                if checkbox(
                    ui,
                    &mut self.settings.ui.transparent,
                    self.i18n.t(I18nKey::Transparent),
                )
                .on_hover_text(self.i18n.t(I18nKey::TransparentHover))
                .changed()
                {
                    set_theme(ui.ctx(), self.settings.ui.theme, self.settings.ui.transparent);
                }

                self.ui_theme_box(ui);

                ui.add_space(8.0);
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

            const THEMES: [(Theme, &str); 3] = [
                (Theme::System, Theme::System.as_str()),
                (Theme::Dark, Theme::Dark.as_str()),
                (Theme::Light, Theme::Light.as_str()),
            ];

            if enum_select(ui, &mut self.settings.ui.theme, &THEMES, Some([120.0, 30.0])).changed()
            {
                set_theme(ui.ctx(), self.settings.ui.theme, self.settings.ui.transparent);
            }
        });
    }

    /// UI
    /// ```txt
    /// |<---label--->|<------ stretch ------>|<-60->|<-60->|
    /// | Skyrim      | path..................| Auto | ...  |
    /// | Output      | path..................|      | ...  |
    /// ```
    pub(crate) fn ui_paths(&mut self, ctx: &egui::Context) {
        let panel = super::themed_top_bottom_panel(
            egui::TopBottomPanel::top("top_paths"),
            self.settings.ui.theme,
            self.settings.ui.transparent,
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
    ui.add(egui_shadcn::Checkbox::new(checked).label(label))
}

const LABEL_WIDTH: f32 = 140.0;
const BUTTON_WIDTH: f32 = 90.0;
const ROW_HEIGHT: f32 = 40.0;
const BUTTON_HEIGHT: f32 = 32.0;
const RIGHT_MARGIN: f32 = 8.0;

fn path_text_edit(
    ui: &mut egui::Ui,
    value: &mut String,
    transparent: bool,
    hint: Option<&str>,
) -> egui::Response {
    let _ = transparent;
    let mut input = egui_shadcn::Input::new(value).desired_width(ui.available_width());

    if let Some(hint) = hint {
        input = input.placeholder(hint);
    }

    input.show(ui)
}
