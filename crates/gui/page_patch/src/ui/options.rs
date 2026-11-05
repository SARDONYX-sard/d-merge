//! The panels above the mod list: execution mode, runtime, parser, options and the directories.

use egui::Label;
use egui_widgets::{
    PathActions, PathRow,
    shadcn::{Checkbox, ComboBox, RadioButton},
};
use gui_shared::{
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

    /// The Skyrim row, with `Auto` in VFS mode (Windows: it is in the registry).
    fn ui_skyrim_dir_row(&mut self, ui: &mut egui::Ui) {
        let i18n = self.ctx.i18n;
        let settings = &mut *self.ctx.settings;
        let is_vfs = settings.behavior.mode == DataMode::Vfs;
        if is_vfs && self.page.first_frame && settings.vfs.skyrim_data_dir.trim().is_empty() {
            self.page.detect_vfs_data_dir(settings, self.ctx.notify, i18n);
            return;
        }

        let mut auto = false;
        let row = PathRow::new(i18n.t(I18nKey::SkyrimDataDirLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
            .hover(i18n.t(I18nKey::OpenSelectedPathHover))
            .hint((!is_vfs).then_some("D:\\GAME\\ModOrganizer Skyrim SE\\mods\\*"))
            .show_with(ui, settings.current_skyrim_data_dir_mut(), |ui| auto = auto_detect_button(ui, is_vfs, i18n));
        if let Some(err) = row.open_error {
            self.ctx.notify.error(err);
        }
        if auto {
            self.page.detect_vfs_data_dir(self.ctx.settings, self.ctx.notify, i18n);
        }
        if self.page.first_frame || row.changed {
            self.page.fetch_mods(self.ctx.settings);
        }
    }

    /// The output row. The empty space lines its field up with the Skyrim row's.
    fn ui_output_dir_row(&mut self, ui: &mut egui::Ui) {
        let i18n = self.ctx.i18n;
        let row = PathRow::new(i18n.t(I18nKey::OutputDirLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
            .hover(i18n.t(I18nKey::OpenSelectedPathHover))
            .show_with(ui, self.ctx.settings.current_output_dir_mut(), |ui| {
                ui.allocate_space(egui::Vec2::from(PathRow::BUTTON_SIZE));
            });
        if let Some(err) = row.open_error {
            self.ctx.notify.error(err);
        }
    }
}

/// See [`PathRow`].
const PATH_ACTIONS: PathActions =
    PathActions { open: gui_shared::fs::open_dir_of, pick: gui_shared::fs::pick_folder_from };

/// `Auto`, where it applies; else its space. Returns whether it was clicked.
fn auto_detect_button(ui: &mut egui::Ui, is_vfs: bool, i18n: &gui_shared::i18n::I18nMap) -> bool {
    if cfg!(target_os = "windows") && is_vfs {
        let button = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::AutoDetectButton))
            .icon(egui_widgets::shadcn::Icon::ScanSearch)
            .full_width();
        ui.add_sized(PathRow::BUTTON_SIZE, button).on_hover_text(i18n.t(I18nKey::AutoDetectHover)).clicked()
    } else {
        ui.allocate_space(egui::Vec2::from(PathRow::BUTTON_SIZE));
        false
    }
}
