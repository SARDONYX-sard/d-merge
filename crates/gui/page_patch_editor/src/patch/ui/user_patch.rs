//! The window of the user patch: its own editor. What it is is in [`crate::patch::user_patch`].
//!
//! ```text
//! ┌ User Patch ─────────────────────────────────────────────────────┐
//! │ D:/out/.d_merge/user_patch       [Open work data] [Delete all]  │
//! │ (the side bar and the editor, as in the Patch Editor)           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use egui_widgets::{
    Modal,
    notify::Notification,
    shadcn::{Button, Icon},
};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::{pages::patch_editor::PatchEditorSettings, ui::editor::LspOptions},
};

use crate::patch::{PatchPaths, PatchTool};

impl PatchTool {
    /// Draws the window if it is open. It belongs to no page: it stays when another is shown.
    pub(crate) fn user_patch_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
        paths: PatchPaths<'_>,
        settings: &mut PatchEditorSettings,
        editor: &mut LspOptions,
    ) {
        if !self.user_patch.open {
            return;
        }
        let to_top = std::mem::take(&mut self.user_patch.to_top);
        let has_output = !paths.patch_output_dir.trim().is_empty();
        let mut tool = has_output.then(|| self.user_patch_tool(paths, notify, i18n));
        let mut open = true;
        Modal::new(i18n.t(I18nKey::UserPatchTitle)).to_top(to_top).show(ctx, &mut open, |ui| match &mut tool {
            Some(tool) => tool.user_patch_ui(ui, i18n, notify, settings, editor),
            None => {
                ui.colored_label(ui.visuals().warn_fg_color, i18n.t(I18nKey::NemesisNeedPatchOutput));
            }
        });
        self.user_patch.open = open;
        if !open {
            self.close_user_patch(notify, i18n);
        }
    }

    /// One frame of the user patch's own tool, which keeps nothing of its work in the settings.
    fn user_patch_ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        notify: &mut Notification,
        settings: &mut PatchEditorSettings,
        editor: &mut LspOptions,
    ) {
        self.read_editor_options(editor);
        self.read_view_settings(settings);
        self.sync_workspace(notify, i18n);
        self.take_in_resolution(notify, i18n);
        self.draw(ui, i18n, notify);
        self.write_editor_options(editor);
        self.write_view_settings(settings);
    }

    /// Where it goes, which is not for the user to change, and deleting all of it.
    pub(in crate::patch) fn user_patch_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.clear_button(ui, i18n, notify);
                self.open_cache_button(ui, i18n, notify);
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let output = egui::RichText::new(self.output.trim()).weak();
                    ui.add(egui::Label::new(output).truncate()).on_hover_text(i18n.t(I18nKey::UserPatchOutputHover));
                });
            });
        });
    }

    /// `[Delete all]`, then `[Delete everything?] [Cancel]`: nothing of it can be undone.
    fn clear_button(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        if !self.confirm_clear {
            let button = Button::new(i18n.t(I18nKey::UserPatchClear)).icon(Icon::Trash).destructive();
            self.confirm_clear = ui.add(button).on_hover_text(i18n.t(I18nKey::UserPatchClearHover)).clicked();
            return;
        }
        if ui.add(Button::new(i18n.t(I18nKey::NemesisCancel))).clicked() {
            self.confirm_clear = false;
        }
        let sure = Button::new(i18n.t(I18nKey::UserPatchClearConfirm)).icon(Icon::Trash).destructive();
        if ui.add(sure).on_hover_text(i18n.t(I18nKey::UserPatchClearHover)).clicked() {
            self.confirm_clear = false;
            self.clear_user_patch(notify, i18n);
        }
    }
}
