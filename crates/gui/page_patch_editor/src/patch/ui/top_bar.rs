//! The bars above the object list: output and mod code, the template picker, the behavior tabs and
//! the patch target.

use std::path::Path;

use egui_widgets::{
    notify::Notification,
    shadcn::{Button, Checkbox, TextEdit},
};
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_valid_mod_code;

use crate::patch::{PatchTool, workspace::Target};

impl PatchTool {
    pub(in crate::patch) fn settings_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.horizontal(|ui| {
            ui.label(i18n.t(I18nKey::NemesisOutputLabel)).on_hover_text(i18n.t(I18nKey::NemesisOutputHover));
            ui.add(TextEdit::singleline(&mut self.output).desired_width(260.0));
            if ui.add(Button::new(i18n.t(I18nKey::SelectButton))).clicked()
                && let Some(dir) = rfd::FileDialog::new().set_directory(&self.output).pick_folder()
            {
                self.output = dir.display().to_string();
            }

            ui.separator();
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
                (I18nKey::NemesisNameLabel, &mut info.name, 140.0),
                (I18nKey::NemesisAuthorLabel, &mut info.author, 90.0),
                (I18nKey::NemesisSiteLabel, &mut info.site, 140.0),
            ] {
                ui.label(i18n.t(label));
                changed |= ui.add(TextEdit::singleline(value).desired_width(width)).changed();
            }
            if changed {
                self.mark_dirty();
            }
        });

        ui.horizontal(|ui| {
            let label = format!("{} ▼", i18n.t(I18nKey::NemesisAddBehavior));
            if ui
                .add(Button::selectable(self.picker_open, label))
                .on_hover_text(format!("{} {}", i18n.t(I18nKey::NemesisTemplatesLabel), self.template_dir))
                .clicked()
            {
                self.picker_open = !self.picker_open;
            }
            if ui
                .add(Button::new(i18n.t(I18nKey::NemesisCustomXml)))
                .on_hover_text(i18n.t(I18nKey::NemesisCustomXmlHover))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Template", &["xml", "bin"])
                    .set_directory(&self.template_dir)
                    .pick_file()
            {
                self.add_behavior(&path, notify, i18n);
            }
            if ui
                .add(Button::new(i18n.t(I18nKey::NemesisImportMod)))
                .on_hover_text(i18n.t(I18nKey::NemesisImportModHover))
                .clicked()
                && let Some(dir) = rfd::FileDialog::new().pick_folder()
            {
                self.import(&dir, notify, i18n);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let ready = self.loaded_for.is_some() && !self.workspace.behaviors.is_empty();
                if ui
                    .add_enabled(ready, Button::new(i18n.t(I18nKey::NemesisExport)).primary())
                    .on_disabled_hover_text(i18n.t(I18nKey::NemesisExportDisabledHover))
                    .clicked()
                {
                    self.export(notify, i18n);
                }
            });
        });
    }

    /// The templates of `template_dir` with a filter; a click adds one as a behavior.
    pub(in crate::patch) fn template_picker(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        if self.catalog_for.as_deref() != Some(self.template_dir.as_str()) {
            self.catalog = crate::patch::workspace::catalog(Path::new(&self.template_dir));
            self.catalog_for = Some(self.template_dir.clone());
        }
        let mut picked = None;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.picker_filter)
                        .hint_text(i18n.t(I18nKey::NemesisFilterHint))
                        .desired_width(240.0),
                );
                ui.weak(format!("{} ({})", self.template_dir, self.catalog.len()));
            });
            if self.catalog.is_empty() {
                ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNoTemplates));
                return;
            }
            // No filter: as a directory tree. A filter wants all the matches at once.
            let clicked = if self.picker_filter.is_empty() {
                super::template_picker::tree_ui(ui, &self.catalog, &mut self.picker_toggled, i18n)
            } else {
                super::template_picker::filtered_ui(ui, &self.catalog, &self.picker_filter, i18n)
            };
            picked = clicked.map(Path::to_path_buf);
        });
        if let Some(path) = picked {
            self.add_behavior(&path, notify, i18n);
            self.picker_open = false;
        }
    }

    /// `[draugrbehavior] [0_master] ...` (`*` = has changes)
    /// Right click a tab: remove that behavior (and its edits) from the work.
    pub(in crate::patch) fn behavior_tabs(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for (i, behavior) in self.workspace.behaviors.iter().enumerate() {
                let count = behavior.edits.len() + behavior.added.len();
                let label = if count > 0 { format!("{} ({count})", behavior.name) } else { behavior.name.clone() };
                let response = ui
                    .add(Button::selectable(i == self.active, label))
                    .on_hover_text(behavior.template_path.display().to_string());
                if response.clicked() {
                    self.active = i;
                }
                response.context_menu(|ui| {
                    let button = Button::new(i18n.t(I18nKey::NemesisRemoveBehavior)).destructive();
                    if ui.add(button).on_hover_text(i18n.t(I18nKey::NemesisRemoveBehaviorHover)).clicked() {
                        remove = Some(i);
                    }
                });
            }
        });
        if let Some(i) = remove {
            self.workspace.behaviors.remove(i);
            self.reset_names(); // the numbers of the others shift
            self.active = self.active.min(self.workspace.behaviors.len().saturating_sub(1));
            self.selected = None;
            self.editor_for = None;
            self.mark_dirty();
        }
    }

    /// `→ Nemesis_Engine/mod/mymod/0_master   ☐ NemesisExt` (+ a warning for NemesisExt)
    pub(in crate::patch) fn target_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let code = if is_valid_mod_code(self.mod_code.trim()) { self.mod_code.trim() } else { "<code>" };
        let behavior = &mut self.workspace.behaviors[self.active];
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("→ {}", behavior.patch_dir(code).display()));
            if behavior.asdsf {
                return; // always `Nemesis_Engine/mod/<code>/animationsetdatasinglefile`
            }
            let supported = behavior.nemesis.is_some();
            let mut ext = behavior.force_ext || !supported;
            let hover = if supported { I18nKey::NemesisForceExtHover } else { I18nKey::NemesisUnsupportedTemplate };
            if ui
                .add_enabled(supported, Checkbox::new(&mut ext, i18n.t(I18nKey::NemesisForceExt)))
                .on_hover_text(i18n.t(hover))
                .on_disabled_hover_text(i18n.t(hover))
                .changed()
            {
                behavior.force_ext = ext;
                changed = true;
            }
            if behavior.target() == Target::Ext {
                ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisExtOnlyWarning));
            }
        });
        if changed {
            self.mark_dirty();
        }
    }
}
