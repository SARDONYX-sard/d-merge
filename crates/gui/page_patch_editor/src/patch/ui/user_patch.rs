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
    HoverMarkdown as _,
    shadcn::{AlertDialog, AlertDialogChoice, Button, Icon},
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
) -> Vec<BarAsk> {
    let mut asks = Vec::new();

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if clear_button(ui, i18n, &mut panels.confirm_clear) {
                asks.push(BarAsk::ClearUserPatch);
            }
            if super::top_bar::cache_button(ui, i18n, &work.place.cache_dir()) {
                asks.push(BarAsk::OpenCache);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let output = egui::RichText::new(work.place.output.trim()).weak();
                ui.add(egui::Label::new(output).truncate()).on_hover_markdown(i18n.t(I18nKey::UserPatchOutputHover));
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
    if ui.add(button).on_hover_markdown(i18n.t(I18nKey::UserPatchClearHover)).clicked() {
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
