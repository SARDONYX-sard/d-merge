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

use egui::Vec2;
use egui_widgets::{
    Modal,
    notify::Notification,
    shadcn::{AlertDialog, AlertDialogChoice, Button, Icon},
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
        Modal::new(i18n.t(I18nKey::UserPatchTitle))
            .size(Vec2::new(0.90, 0.95))
            .to_top(to_top)
            // Its editor uses `Esc` (Vim, hovers, quick fixes): the window closes with its `x`.
            .close_on_escape(false)
            .show(ctx, &mut open, |ui| match &mut tool {
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
        self.mod_export_window(ui.ctx(), i18n, notify);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.clear_button(ui, i18n, notify);
                self.open_cache_button(ui, i18n, notify);
                let export = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::UserPatchExportMod));
                if ui.add(export).on_hover_text(i18n.t(I18nKey::UserPatchExportModHover)).clicked() {
                    self.mod_export.open = true;
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let output = egui::RichText::new(self.output.trim()).weak();
                    ui.add(egui::Label::new(output).truncate()).on_hover_text(i18n.t(I18nKey::UserPatchOutputHover));
                });
            });
        });
    }

    /// `[Delete all]`, which asks first: nothing of it can be undone.
    /// `Output: [...] [Select]  Format: [Nemesis Ext ▼]  [Export]`: see `user_patch::ModExport`.
    fn mod_export_window(&mut self, ctx: &egui::Context, i18n: &I18nMap, notify: &mut Notification) {
        use crate::patch::workspace::ExportFormat;
        if !self.mod_export.open {
            return;
        }
        let label = |format: ExportFormat| {
            ExportFormat::ALL.iter().find(|(known, _)| *known == format).map_or("", |(_, label)| *label)
        };
        let (mut open, mut export) = (true, false);
        let window = egui_widgets::window_near_pointer(ctx, i18n.t(I18nKey::UserPatchExportMod)).open(&mut open);
        window.show(ctx, |ui| {
            let path = super::top_bar::PATH_ACTIONS;
            let row = egui_widgets::PathRow::new(i18n.t(I18nKey::OutputLabel), i18n.t(I18nKey::SelectButton), path)
                .help(i18n.t(I18nKey::UserPatchExportModHover))
                .show(ui, &mut self.mod_export.output);
            if let Some(err) = row.open_error {
                notify.error(err);
            }
            ui.horizontal(|ui| {
                ui.label(i18n.t(I18nKey::OutputFormatLabel));
                let combo = egui::ComboBox::from_id_salt("user_patch_mod_format");
                combo.selected_text(label(self.mod_export.format)).show_ui(ui, |ui| {
                    for &(format, text) in &ExportFormat::ALL {
                        ui.selectable_value(&mut self.mod_export.format, format, text);
                    }
                });
            });
            if self.mod_export.format == ExportFormat::Nemesis {
                self.outside_nemesis_note(ui, i18n);
            }
            let ready = !self.mod_export.output.trim().is_empty();
            let button = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::NemesisExport)).primary();
            export = ui.add_enabled(ready, button).clicked();
        });
        if export {
            self.export_user_patch_as_mod(notify, i18n);
        }
        self.mod_export.open = open && !export;
    }

    /// `⚠ Nemesis does not know these templates: ... (2)`, the templates on hover. Nothing if all
    /// are ones Nemesis knows. See `user_patch::ModExport`.
    fn outside_nemesis_note(&self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let count = self.workspace.outside_nemesis().count();
        if count == 0 {
            return;
        }
        let text = format!("⚠ {} ({count})", i18n.t(I18nKey::UserPatchOutsideNemesis));
        let note = ui.colored_label(ui.visuals().warn_fg_color, text);
        note.on_hover_ui(|ui| {
            for template in self.workspace.outside_nemesis() {
                ui.label(template);
            }
        });
    }

    fn clear_button(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        let button = Button::new(i18n.t(I18nKey::UserPatchClear)).icon(Icon::Trash).destructive();
        if ui.add(button).on_hover_text(i18n.t(I18nKey::UserPatchClearHover)).clicked() {
            self.confirm_clear = true;
        }
        if !self.confirm_clear {
            return;
        }
        let dialog = AlertDialog::new(
            i18n.t(I18nKey::UserPatchClearConfirm),
            i18n.t(I18nKey::CancelButton),
            i18n.t(I18nKey::UserPatchClear),
        );
        let choice = dialog.description(i18n.t(I18nKey::UserPatchClearHover)).action_icon(Icon::Trash).destructive();
        let choice = choice.show(ui.ctx());
        self.confirm_clear = choice.is_none();
        if choice == Some(AlertDialogChoice::Action) {
            self.clear_user_patch(notify, i18n);
        }
    }
}
