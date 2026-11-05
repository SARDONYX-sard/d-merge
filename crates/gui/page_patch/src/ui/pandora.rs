//! The window to take the settings of Pandora Behaviour Engine, or give them to it.
//!
//! ```text
//! Target version: [v4.4.0-beta v]
//! [output dir] [D:/Pandora             ] [Select]
//! [Import] [Export]
//! ```
//! It is rarely needed, so it stays out of the way of the page. Only the VFS mode's settings are
//! concerned, see [`Pandora`].

use egui_widgets::{
    PathRow,
    shadcn::{Button, ComboBox, Icon},
};
use gui_shared::{
    i18n::I18nKey,
    settings::pages::patch::support_pandora::{Pandora, PandoraVersion},
};

use super::{PatchView, options::PATH_ACTIONS};

const WINDOW_WIDTH: f32 = 620.0;

impl PatchView<'_> {
    /// The button that opens the window.
    pub(super) fn pandora_button(&mut self, ui: &mut egui::Ui) {
        let button = Button::new("Pandora").icon(Icon::FolderSearch).full_width();
        if ui.add_sized([120.0, 40.0], button).on_hover_text(self.ctx.i18n.t(I18nKey::PandoraButtonHover)).clicked() {
            self.page.pandora_open ^= true;
        }
    }

    pub(super) fn ui_pandora_window(&mut self, ctx: &egui::Context) {
        if !self.page.pandora_open {
            return;
        }
        let mut open = !ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let window = egui_widgets::window_near_pointer(ctx, "Pandora").open(&mut open);
        window.collapsible(false).default_width(WINDOW_WIDTH).resizable([true, false]).show(ctx, |ui| {
            self.pandora_form(ui);
        });
        self.page.pandora_open = open;
    }

    fn pandora_form(&mut self, ui: &mut egui::Ui) {
        let i18n = self.ctx.i18n;
        let pandora = &mut self.ctx.settings.pandora;
        ui.horizontal(|ui| {
            let hover = i18n.t(I18nKey::PandoraTargetVersionHover);
            ui.label(i18n.t(I18nKey::PandoraTargetVersionLabel)).on_hover_text(hover);
            version_box(ui, &mut pandora.version, hover);
        });
        let row = PathRow::new(i18n.t(I18nKey::PandoraOutputDirLabel), i18n.t(I18nKey::SelectButton), PATH_ACTIONS)
            .help(i18n.t(I18nKey::PandoraDirHover))
            .show(ui, &mut pandora.output_dir);
        if let Some(err) = row.open_error {
            self.ctx.notify.error(err);
        }

        let (mut import, mut export) = (false, false);
        ui.horizontal(|ui| {
            let button =
                |ui: &mut egui::Ui, label, hover| ui.add(Button::new(i18n.t(label))).on_hover_text(i18n.t(hover));
            import = button(ui, I18nKey::PandoraImportLabel, I18nKey::PandoraImportHover).clicked();
            export = button(ui, I18nKey::PandoraExportLabel, I18nKey::PandoraExportHover).clicked();
        });
        if import {
            self.import_pandora();
        } else if export {
            self.export_pandora();
        }
    }

    fn import_pandora(&mut self) {
        let settings = &mut *self.ctx.settings;
        let dir = settings.pandora.output_dir.clone();
        match Pandora::import_to(&dir, settings, settings.pandora.version) {
            Ok(()) => {
                self.ctx.notify.success(format!("Imported Pandora settings from {dir}"));
                // The data directory may be another one now.
                self.page.fetch_mods(self.ctx.settings);
            }
            Err(err) => self.ctx.notify.error(err),
        }
    }

    fn export_pandora(&mut self) {
        let settings = &*self.ctx.settings;
        let dir = &settings.pandora.output_dir;
        match Pandora::export_from(dir, settings, settings.pandora.version) {
            Ok(()) => self.ctx.notify.success(format!("Exported Pandora settings to {dir}")),
            Err(err) => self.ctx.notify.error(err),
        }
    }
}

/// What Pandora's files look like depends on its version.
fn version_box(ui: &mut egui::Ui, version: &mut PandoraVersion, hover: &str) {
    let versions = [PandoraVersion::V4_4_0Beta];
    let mut index = versions.iter().position(|known| known == version).unwrap_or(0);
    let picked = ComboBox::from_id_salt("pandora_version")
        .width(120.0)
        .show_index(ui, &mut index, versions.len(), |i| versions[i].to_static_str())
        .on_hover_text(hover);
    if picked.changed() {
        *version = versions[index];
    }
}
