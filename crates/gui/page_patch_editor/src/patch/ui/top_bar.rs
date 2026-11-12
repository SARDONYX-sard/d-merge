//! The bar above everything: what is being made, and exporting it. Its details fold away, as they
//! are set once.
//!
//! ```text
//! [File v] > mymod  D:/mods/out                [Open work data] | [Nemesis v] [Export]
//! ```
//! Unfolded: where it is exported to, the mod it was read from, the mods it depends on, and the mod's
//! `info.ini`. The list of behaviors is in [`super::behaviors`].

use std::path::Path;

use egui_widgets::{
    PathActions, PathRow,
    notify::Notification,
    shadcn::{Button, ComboBox, Icon, TextEdit},
};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::RecentWork,
};

use crate::patch::{
    PatchTool,
    new_work::NewWork,
    workspace::{ExportFormat, Workspace},
};

/// The width of the box of [`PatchTool::export_format_box`].
const EXPORT_FORMAT_WIDTH: f32 = 112.0;

/// See [`PathRow`].
pub(super) const PATH_ACTIONS: PathActions =
    PathActions { open: gui_shared::fs::open_dir_of, pick: gui_shared::fs::pick_folder_from };

/// What was picked in the `File` menu.
#[derive(Debug)]
enum FileAction {
    New,
    Open,
    Recent(RecentWork),
    Close,
}

impl PatchTool {
    /// The one line, and the details if unfolded.
    pub(in crate::patch) fn project_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.horizontal(|ui| {
            self.file_menu(ui, i18n, notify);
            let chevron = if self.project_open { Icon::ChevronDown } else { Icon::ChevronRight };
            let mut toggle = ui.add(Button::new("").icon(chevron).small()).clicked();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.export_button(ui, i18n, notify);
                self.open_cache_button(ui, i18n, notify);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    toggle |= self.project_summary(ui);
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

    /// `New`, `Open`, `Open Recent`, `Close`, like VS Code's.
    fn file_menu(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let mut action = None;
        // With a frame and an arrow: it should be seen to be a menu.
        let button = egui::Button::new(format!("{} ▾", i18n.t(I18nKey::NemesisFileMenu)));
        egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
            let mut item = |ui: &mut egui::Ui, label, hover, picked| {
                if ui.button(i18n.t(label)).on_hover_text(i18n.t(hover)).clicked() {
                    action = Some(picked);
                    ui.close();
                }
            };
            item(ui, I18nKey::NemesisNewWork, I18nKey::NemesisNewWorkHover, FileAction::New);
            item(ui, I18nKey::NemesisOpenMod, I18nKey::NemesisOpenModHover, FileAction::Open);
            ui.menu_button(i18n.t(I18nKey::NemesisOpenRecent), |ui| {
                if let Some(work) = recent_list(ui, &self.recent, i18n) {
                    action = Some(FileAction::Recent(work));
                    ui.close();
                }
            });
            ui.separator();
            if ui
                .button(i18n.t(I18nKey::NemesisCloseWork))
                .on_hover_text(i18n.t(I18nKey::NemesisCloseWorkHover))
                .clicked()
            {
                action = Some(FileAction::Close);
                ui.close();
            }
        });
        match action {
            Some(FileAction::New) => {
                // Usually next to the one that was open.
                let output = self.output.clone();
                self.close_work(notify, i18n);
                self.new_work = Some(NewWork { code: String::new(), output, ..NewWork::default() });
            }
            Some(FileAction::Open) => self.pick_mod(notify, i18n),
            Some(FileAction::Recent(work)) => self.open_recent(&work, notify, i18n),
            Some(FileAction::Close) => self.close_work(notify, i18n),
            None => {}
        }
    }

    fn export_button(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let ready = !self.output.trim().is_empty() && !self.workspace.behaviors.is_empty();
        if ui
            .add_enabled(ready, Button::new(i18n.t(I18nKey::NemesisExport)).primary())
            .on_disabled_hover_text(i18n.t(I18nKey::NemesisExportDisabledHover))
            .clicked()
        {
            self.export(notify, i18n);
        }
        self.export_format_box(ui, i18n);
        ui.separator();
    }

    /// What `Export` writes. Beside it, as it belongs to it and not to what opens something.
    fn export_format_box(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let format = self.workspace.export_format;
        let mut selected = ExportFormat::ALL.iter().position(|(known, _)| *known == format).unwrap_or(0);
        let response = ComboBox::from_id_salt("nemesis_export_format").width(EXPORT_FORMAT_WIDTH).show_index(
            ui,
            &mut selected,
            ExportFormat::ALL.len(),
            |index| ExportFormat::ALL[index].1,
        );
        let response = response.on_hover_text(i18n.t(I18nKey::NemesisExportFormatHover));
        if response.changed() {
            self.workspace.export_format = ExportFormat::ALL[selected].0;
            self.preview = None; // it says what the export would write
            self.mark_dirty();
        }
    }

    /// `mymod  D:/mods/out`. Returns whether it was clicked.
    fn project_summary(&self, ui: &mut egui::Ui) -> bool {
        let name = egui::RichText::new(self.mod_code.trim()).strong();
        let mut clicked = ui.add(egui::Label::new(name).sense(egui::Sense::click())).clicked();
        let output = egui::RichText::new(self.output.trim()).weak();
        clicked |= ui.add(egui::Label::new(output).truncate().sense(egui::Sense::click())).clicked();
        clicked
    }

    /// The directories, and the mod's `info.ini`.
    fn project_form(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let output = PathRow::new(i18n.t(I18nKey::NemesisOutputLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
            .help(i18n.t(I18nKey::NemesisOutputHover))
            .show(ui, &mut self.output);
        if let Some(err) = output.open_error {
            notify.error(err);
        }
        if output.changed {
            self.mark_dirty();
        }
        self.source_row(ui, i18n, notify);
        self.ref_mods_row(ui, i18n);

        ui.horizontal_wrapped(|ui| {
            // Fixed: it is in the ids of the new objects and the names of the folders.
            let hover = i18n.t(I18nKey::NemesisModCodeHover);
            ui.label(i18n.t(I18nKey::NemesisModCodeLabel)).on_hover_text(hover);
            ui.strong(self.mod_code.trim()).on_hover_text(hover);

            let info = &mut self.workspace.info;
            let mut changed = false;
            for (label, hover, value, width) in [
                (I18nKey::NemesisNameLabel, I18nKey::NemesisNameHover, &mut info.name, 260.0),
                (I18nKey::NemesisAuthorLabel, I18nKey::NemesisAuthorHover, &mut info.author, 130.0),
                (I18nKey::NemesisSiteLabel, I18nKey::NemesisSiteHover, &mut info.site, 320.0),
                (I18nKey::NemesisVersionLabel, I18nKey::NemesisVersionHover, &mut info.version, 70.0),
            ] {
                ui.label(i18n.t(label)).on_hover_text(i18n.t(hover));
                let field = ui.add(TextEdit::singleline(value).desired_width(width));
                changed |= field.on_hover_text(i18n.t(hover)).changed();
            }
            if changed {
                self.mark_dirty();
            }
        });
    }

    /// `[Read from] D:/mods/Paraglider  [Reload]`, if the work was read from a mod.
    fn source_row(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        if self.mod_dir.trim().is_empty() {
            return;
        }
        let mut reload = false;
        let error = PathRow::new(i18n.t(I18nKey::NemesisModDirLabel), "", PATH_ACTIONS)
            .hover(i18n.t(I18nKey::NemesisSourceHover))
            .show_read_only(ui, self.mod_dir.trim(), |ui| {
                let button = Button::new(i18n.t(I18nKey::NemesisReload)).icon(Icon::RefreshCcw);
                reload = ui.add(button).on_hover_text(i18n.t(I18nKey::NemesisReloadHover)).clicked();
            });
        if let Some(err) = error {
            notify.error(err);
        }
        if reload {
            self.reload_from_mod(notify, i18n);
        }
    }

    /// `Open work data`, always in reach. Its hover says where that is.
    pub(super) fn open_cache_button(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let button = Button::new(i18n.t(I18nKey::NemesisOpenCacheDir)).icon(Icon::FolderOpen);
        let response = ui.add(button).on_hover_ui(|ui| {
            ui.label(i18n.t(I18nKey::NemesisCacheDirHover));
            ui.monospace(self.cache_dir().to_string_lossy());
        });
        if response.clicked() {
            // A new work has no directory until its first save.
            self.save_now(notify, i18n);
            if let Err(err) = gui_shared::fs::open_dir_of(&self.cache_dir().to_string_lossy()) {
                notify.error(err);
            }
        }
    }

    /// The directory of the open work, or of all the works if none is open.
    fn cache_dir(&self) -> std::path::PathBuf {
        let root = &self.cache_root;
        let dir = self.loaded_for.as_deref().map_or_else(|| root.clone(), |code| Workspace::cache_dir(root, code));
        std::path::absolute(&dir).unwrap_or(dir)
    }

    /// `References: [amco x] [+]`: the mods this one depends on, see `Workspace::ref_mods`.
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

/// The works opened lately. Returns the one clicked.
pub(super) fn recent_list(ui: &mut egui::Ui, recent: &[RecentWork], i18n: &I18nMap) -> Option<RecentWork> {
    if recent.is_empty() {
        ui.weak(i18n.t(I18nKey::NemesisOpenRecentEmpty));
    }
    let mut picked = None;
    for work in recent {
        if ui.button(format!("{}  {}", work.mod_code, work.output)).clicked() {
            picked = Some(work.clone());
        }
    }
    picked
}
