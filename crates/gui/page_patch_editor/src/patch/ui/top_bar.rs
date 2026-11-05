//! The bar above everything: what is being made, and exporting it. Its details fold away, as they
//! are set once.
//!
//! ```text
//! ▶ mymod  D:/mods/out                                        [Export]
//! ```
//! Unfolded (also at the start while something is missing): the output, the mod to read, the mods
//! it depends on, and the mod's `info.ini`. The list of behaviors is in [`super::behaviors`].

use std::path::{Path, PathBuf};

use egui_widgets::{
    PathActions, PathRow,
    notify::Notification,
    shadcn::{Button, Icon, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_valid_mod_code;

use crate::patch::PatchTool;

/// See [`PathRow`].
const PATH_ACTIONS: PathActions =
    PathActions { open: gui_shared::fs::open_dir_of, pick: gui_shared::fs::pick_folder_from };

impl PatchTool {
    /// The one line, and the details if unfolded.
    pub(in crate::patch) fn project_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.horizontal(|ui| {
            let chevron = if self.project_open { Icon::ChevronDown } else { Icon::ChevronRight };
            let mut toggle = ui.add(Button::new("").icon(chevron).small()).clicked();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.export_button(ui, i18n, notify);
                self.open_recent_menu(ui, i18n);
                if ui
                    .add(Button::new(i18n.t(I18nKey::UserPatchTitle)))
                    .on_hover_text(i18n.t(I18nKey::UserPatchHover))
                    .clicked()
                {
                    self.toggle_user_patch();
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    toggle |= self.project_summary(ui, i18n);
                });
            });
            if toggle {
                self.project_open = !self.project_open;
            }
        });
        if self.project_open {
            self.project_form(ui, i18n, notify);
        }
    }

    /// `Open Recent`, like VS Code's: a work comes back with the tabs it had open.
    fn open_recent_menu(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let mut picked = None;
        ui.menu_button(i18n.t(I18nKey::NemesisOpenRecent), |ui| {
            if self.recent.is_empty() {
                ui.weak(i18n.t(I18nKey::NemesisOpenRecentEmpty));
            }
            for work in &self.recent {
                if ui.button(format!("{}  {}", work.mod_code, work.output)).clicked() {
                    picked = Some(work.clone());
                    ui.close();
                }
            }
        });
        if let Some(work) = picked {
            self.open_recent(&work);
        }
    }

    fn export_button(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let ready = self.loaded_for.is_some() && !self.workspace.behaviors.is_empty();
        if ui
            .add_enabled(ready, Button::new(i18n.t(I18nKey::NemesisExport)).primary())
            .on_disabled_hover_text(i18n.t(I18nKey::NemesisExportDisabledHover))
            .clicked()
        {
            self.export(notify, i18n);
        }
    }

    /// `mymod  D:/mods/out`, or `✖` if the code is not valid. Returns whether it was clicked.
    fn project_summary(&self, ui: &mut egui::Ui, i18n: &I18nMap) -> bool {
        let code = self.mod_code.trim();
        let mut clicked = false;
        if !is_valid_mod_code(code) {
            ui.colored_label(ui.visuals().error_fg_color, "✖").on_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid));
        }
        let name = if code.is_empty() { i18n.t(I18nKey::NemesisModCodeLabel) } else { code };
        clicked |= ui.add(egui::Label::new(egui::RichText::new(name).strong()).sense(egui::Sense::click())).clicked();
        let output = egui::RichText::new(self.output.trim()).weak();
        clicked |= ui.add(egui::Label::new(output).truncate().sense(egui::Sense::click())).clicked();
        clicked
    }

    /// The output, the mod to read and the mod's `info.ini`.
    fn project_form(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let select = i18n.t(I18nKey::SelectButton);
        let output = PathRow::new(i18n.t(I18nKey::NemesisOutputLabel), select, PATH_ACTIONS)
            .hover(i18n.t(I18nKey::NemesisOutputHover))
            .show(ui, &mut self.output);
        if let Some(err) = output.open_error {
            notify.error(err);
        }
        let mod_row = PathRow::new(i18n.t(I18nKey::NemesisModDirLabel), select, PATH_ACTIONS)
            .hover(i18n.t(I18nKey::NemesisImportModHover))
            .show(ui, &mut self.mod_dir);
        if let Some(err) = mod_row.open_error {
            notify.error(err);
        }
        // Reading a mod replaces the work: only on a choice, or `Enter` in the field.
        if (mod_row.picked || mod_row.submitted) && !self.mod_dir.trim().is_empty() {
            let dir = PathBuf::from(self.mod_dir.trim());
            self.import(&dir, notify, i18n);
        }

        self.ref_mods_row(ui, i18n);

        ui.horizontal_wrapped(|ui| {
            ui.label(i18n.t(I18nKey::NemesisModCodeLabel));
            let code = ui.add(
                TextEdit::singleline(&mut self.mod_code)
                    .hint_text(i18n.t(I18nKey::NemesisModCodeHint))
                    .desired_width(70.0),
            );
            if !self.mod_code.is_empty() && !is_valid_mod_code(self.mod_code.trim()) {
                code.on_hover_text(i18n.t(I18nKey::NemesisModCodeInvalid));
                ui.colored_label(ui.visuals().error_fg_color, "✖");
            }

            let info = &mut self.workspace.info;
            let mut changed = false;
            for (label, value, width) in [
                (I18nKey::NemesisNameLabel, &mut info.name, 260.0),
                (I18nKey::NemesisAuthorLabel, &mut info.author, 130.0),
                (I18nKey::NemesisSiteLabel, &mut info.site, 320.0),
            ] {
                ui.label(i18n.t(label));
                changed |= ui.add(TextEdit::singleline(value).desired_width(width)).changed();
            }
            if changed {
                self.mark_dirty();
            }
        });
    }

    /// `References: [amco ✖] [+]`: the mods this one depends on, see `Workspace::ref_mods`.
    fn ref_mods_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let mut remove = None;
        let mut add = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(i18n.t(I18nKey::NemesisRefMods)).on_hover_text(i18n.t(I18nKey::NemesisRefModsHover));
            for (at, dir) in self.workspace.ref_mods.iter().enumerate() {
                let code =
                    Path::new(dir).file_name().map_or_else(|| dir.as_str().into(), |name| name.to_string_lossy());
                if ui.add(Button::new(format!("{code} ✖")).small()).on_hover_text(dir).clicked() {
                    remove = Some(at);
                }
            }
            add = ui
                .add(Button::new("").icon(Icon::Plus).small())
                .on_hover_text(i18n.t(I18nKey::NemesisRefModsHover))
                .clicked();
        });
        if let Some(at) = remove {
            self.workspace.ref_mods.remove(at);
            self.workspace.reload_refs();
        } else if add && let Some(dir) = gui_shared::rfd::FileDialog::new().pick_folder() {
            let code = self.mod_code.trim().to_owned();
            self.workspace.add_ref(&dir, &code);
        } else {
            return;
        }
        self.reset_names();
        self.editor_for = None;
        self.mark_dirty();
    }
}
