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
use gui_shared::i18n::{I18nKey, I18nMap};

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
    if panels.mod_export.window(ui.ctx(), i18n, notify, &outside) {
        asks.push(BarAsk::ExportUserPatchAsMod);
    }

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if clear_button(ui, i18n, &mut panels.confirm_clear) {
                asks.push(BarAsk::ClearUserPatch);
            }
            if super::top_bar::cache_button(ui, i18n, &work.place.cache_dir()) {
                asks.push(BarAsk::OpenCache);
            }
            let export = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::UserPatchExportMod));
            if ui.add(export).on_hover_text(i18n.t(I18nKey::UserPatchExportModHover)).clicked() {
                panels.mod_export.open = true;
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let output = egui::RichText::new(work.place.output.trim()).weak();
                ui.add(egui::Label::new(output).truncate()).on_hover_text(i18n.t(I18nKey::UserPatchOutputHover));
            });
        });
    });
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

impl crate::patch::user_patch::ModExport {
    /// Draws the window, if open: `Output: [...] [Select]  Format: [Nemesis Ext ▼]  [Export]`, and a
    /// note of the templates Nemesis does not know (`outside`) for its format. Returns whether to
    /// export.
    pub(super) fn window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
        outside: &[&str],
    ) -> bool {
        use crate::patch::workspace::ExportFormat;
        if !self.open {
            return false;
        }

        let (mut open, mut export) = (true, false);
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::UserPatchExportMod)).open(&mut open);
        window.show(ctx, |ui| {
            let path = super::top_bar::PATH_ACTIONS;
            let row = egui_widgets::PathRow::new(i18n.t(I18nKey::OutputLabel), i18n.t(I18nKey::SelectButton), path)
                .help(i18n.t(I18nKey::UserPatchExportModHover))
                .show(ui, &mut self.output);
            if let Some(err) = row.open_error {
                notify.error(err);
            }

            ui.horizontal(|ui| {
                ui.label(i18n.t(I18nKey::OutputFormatLabel));
                let label =
                    ExportFormat::ALL.iter().find(|(known, _)| *known == self.format).map_or("", |(_, label)| *label);
                ComboBox::from_id_salt("user_patch_mod_format").selected_text(label).show_ui(ui, |ui| {
                    for &(format, text) in &ExportFormat::ALL {
                        ui.selectable_value(&mut self.format, format, text);
                    }
                });
            });
            if self.format == ExportFormat::Nemesis {
                outside_nemesis_note(ui, i18n, outside);
            }

            let ready = !self.output.trim().is_empty();
            let button = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::NemesisExport)).primary();
            export = ui.add_enabled(ready, button).clicked();
        });

        self.open = open && !export;
        export
    }
}

/// `⚠ Nemesis does not know these templates: ... (2)`, the templates on hover. Nothing if all are
/// ones Nemesis knows. See `user_patch::ModExport`.
fn outside_nemesis_note(ui: &mut egui::Ui, i18n: &I18nMap, outside: &[&str]) {
    if outside.is_empty() {
        return;
    }
    let text = format!("⚠ {} ({})", i18n.t(I18nKey::UserPatchOutsideNemesis), outside.len());
    ui.colored_label(ui.visuals().warn_fg_color, text).on_hover_ui(|ui| {
        for template in outside {
            ui.label(*template);
        }
    });
}
