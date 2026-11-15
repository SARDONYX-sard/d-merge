//! The window of the user patch: its own editor. What it is is in [`crate::patch::user_patch`].
//!
//! ```text
//! ┌ User Patch ─────────────────────────────────────────────────────┐
//! │ D:/out/.d_merge/user_patch       [Open work data] [Delete all]  │
//! │ (the side bar and the editor, as in the Patch Editor)           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! `Delete all` asks first in an [`AlertDialog`], as it can't be undone.

use egui_widgets::{
    notify::Notification,
    shadcn::{AlertDialog, AlertDialogChoice, Button, ComboBox, Icon},
};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::pages::patch_editor::OutsideNemesis,
};

use super::top_bar::BarAsk;
use crate::patch::{tool::work_state::Work, ui::panels::Panels};

/// Where it goes, which is not for the user to change, and deleting all of it. Returns what was
/// asked: see [`super::top_bar::apply`].
pub(in crate::patch) fn user_patch_bar(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (work, panels): (&Work, &mut Panels),
    notify: &mut Notification,
) -> Vec<BarAsk> {
    let mut asks = Vec::new();
    let outside: Vec<&str> = work.workspace.outside_nemesis().collect();
    asks.extend(panels.mod_export.window(ui.ctx(), i18n, notify, &outside));

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if clear_button(ui, i18n, &mut panels.confirm_clear) {
                asks.push(BarAsk::ClearUserPatch);
            }
            if super::top_bar::cache_button(ui, i18n, &work.place.cache_dir()) {
                asks.push(BarAsk::OpenCache);
            }
            let export = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::UserPatchAsMod));
            if ui.add(export).on_hover_text(i18n.t(I18nKey::UserPatchAsModHover)).clicked() {
                panels.mod_export.open = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let output = egui::RichText::new(work.place.output.trim()).weak();
                ui.add(egui::Label::new(output).truncate()).on_hover_text(i18n.t(I18nKey::UserPatchOutputHover));
            });
        });
    });
    // What it depends on: the mods its values were taken from, whose objects and names it uses.
    asks.extend(super::top_bar::ref_mods_row(ui, i18n, &work.workspace.ref_mods));
    asks
}

/// `[Delete all]`, which asks first: nothing of it can be undone. `confirm`: the question is open.
/// Returns whether it was confirmed.
fn clear_button(ui: &mut egui::Ui, i18n: &I18nMap, confirm: &mut bool) -> bool {
    let button = Button::new(i18n.t(I18nKey::UserPatchClear)).icon(Icon::Trash).destructive();
    if ui.add(button).on_hover_text(i18n.t(I18nKey::UserPatchClearHover)).clicked() {
        *confirm = true;
    }
    if !*confirm {
        return false;
    }

    let dialog = AlertDialog::new(
        i18n.t(I18nKey::UserPatchClearConfirm),
        i18n.t(I18nKey::CancelButton),
        i18n.t(I18nKey::UserPatchClear),
    );
    let choice = dialog.description(i18n.t(I18nKey::UserPatchClearHover)).action_icon(Icon::Trash).destructive();
    let choice = choice.show(ui.ctx());
    *confirm = choice.is_none();
    choice == Some(AlertDialogChoice::Action)
}

/// The export / import window is at least this wide: its path and the format's list fit on their
/// rows, and its description does not wrap at every few words.
const MOD_WINDOW_WIDTH: f32 = 560.0;

impl crate::patch::user_patch::ModExport {
    /// Draws the window, if open: where, the format, a note of the templates Nemesis does not know
    /// (`outside`), `[Export]`, and `[Import (replace)] [Import (add)]` to read the mod there back.
    /// Returns what was asked.
    pub(super) fn window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
        outside: &[&str],
    ) -> Option<BarAsk> {
        use crate::patch::workspace::ExportFormat;
        if !self.open {
            return None;
        }

        let (mut open, mut asked) = (true, None);
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::UserPatchAsMod))
            .min_width(MOD_WINDOW_WIDTH)
            .open(&mut open);
        window.show(ctx, |ui| {
            ui.label(i18n.t(I18nKey::UserPatchAsModHover));
            self.place_ui(ui, i18n, notify);

            {
                let mut selected = ExportFormat::ALL.iter().position(|known| *known == self.format).unwrap_or(0);
                let label = i18n.t(I18nKey::OutputFormatLabel);
                ComboBox::new("user_patch_mod_format", label).label_first().show_index(
                    ui,
                    &mut selected,
                    ExportFormat::ALL.len(),
                    |at| ExportFormat::ALL[at].label(),
                );
                self.format = ExportFormat::ALL[selected];
            }
            if let Some(dir) = self.patch_dir() {
                let to = egui::RichText::new(format!("→ {}", dir.display())).monospace().weak();
                ui.add(egui::Label::new(to).truncate());
            }
            if self.format == ExportFormat::Nemesis {
                self.outside_nemesis_ui(ui, i18n);
                outside_nemesis_note(ui, i18n, outside);
            }

            let ready = self.mod_root().is_some();
            asked = ui.horizontal(|ui| import_export_buttons(ui, i18n, ready)).inner;
        });

        self.open = open && asked.is_none();
        asked
    }
}

impl crate::patch::user_patch::ModExport {
    /// `Output dir: [D:/MO2/mods/My Patch] [Select]`: the mod's root.
    fn place_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let path = super::top_bar::PATH_ACTIONS;
        let label = i18n.t(I18nKey::OutputDirLabel);
        let row = egui_widgets::PathRow::new(label, i18n.t(I18nKey::SelectButton), path)
            .help(i18n.t(I18nKey::ModOutputDirHover))
            .show(ui, &mut self.place.output);
        if let Some(err) = row.open_error {
            notify.error(err);
        }
    }

    /// `Templates Nemesis does not know: [Write to Nemesis_EngineExt ▼]`: see [`OutsideNemesis`].
    fn outside_nemesis_ui(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let all = OutsideNemesis::ALL;
        let mut selected = all.iter().position(|known| *known == self.place.outside_nemesis).unwrap_or(0);
        let label = i18n.t(I18nKey::OutsideNemesisLabel);
        ComboBox::new("user_patch_outside_nemesis", label).label_first().show_index(
            ui,
            &mut selected,
            all.len(),
            |at| i18n.t(all[at].i18n_key()),
        );
        self.place.outside_nemesis = all[selected];
    }
}

/// `[Export] [Import (replace)] [Import (add)]`, enabled once there is a place (`ready`).
fn import_export_buttons(ui: &mut egui::Ui, i18n: &I18nMap, ready: bool) -> Option<BarAsk> {
    let export = Button::new(i18n.t(I18nKey::ExportButton)).primary();
    if ui.add_enabled(ready, export).clicked() {
        return Some(BarAsk::ExportUserPatchAsMod);
    }
    for (replace, text, hover) in [
        (true, I18nKey::ImportReplace, I18nKey::ImportReplaceHover),
        (false, I18nKey::ImportAppend, I18nKey::ImportAppendHover),
    ] {
        if ui.add_enabled(ready, Button::new(i18n.t(text))).on_hover_text(i18n.t(hover)).clicked() {
            return Some(BarAsk::ImportUserPatch { replace });
        }
    }
    None
}

/// `⚠ Templates Nemesis does not know: 2`, the templates on hover: what the setting above is for.
/// Nothing if all are ones Nemesis knows. See `user_patch::ModExport`.
fn outside_nemesis_note(ui: &mut egui::Ui, i18n: &I18nMap, outside: &[&str]) {
    if outside.is_empty() {
        return;
    }
    let text = format!("⚠ {} {}", i18n.t(I18nKey::OutsideNemesisLabel), outside.len());
    ui.colored_label(ui.visuals().warn_fg_color, text).on_hover_ui(|ui| {
        for template in outside {
            ui.label(*template);
        }
    });
}
