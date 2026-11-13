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
    shadcn::{AlertDialog, AlertDialogChoice, Button, ComboBox, Icon},
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
        (settings, editor): (&mut PatchEditorSettings, &mut LspOptions),
        keys: &mut keybinding::Keybindings,
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
                Some(tool) => tool.user_patch_ui(ui, i18n, notify, (settings, editor), keys),
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
        (settings, editor): (&mut PatchEditorSettings, &mut LspOptions),
        keys: &mut keybinding::Keybindings,
    ) {
        self.pane.read_options(editor);
        self.read_view_settings(settings);
        self.sync_workspace(notify, i18n);
        self.take_in_resolution(notify, i18n);
        self.take_key_commands(ui.ctx(), Some(keys), i18n, notify);
        self.draw(ui, i18n, notify);
        self.pane.write_options(editor);
        self.write_view_settings(settings);
    }

    /// Where it goes, which is not for the user to change, and deleting all of it.
    pub(in crate::patch) fn user_patch_bar(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, notify: &mut Notification) {
        self.mod_export_window(ui.ctx(), i18n, notify);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if clear_button(ui, i18n, &mut self.panels.confirm_clear) {
                    self.clear_user_patch(notify, i18n);
                }
                if super::top_bar::cache_button(ui, i18n, &self.cache_dir()) {
                    self.open_cache(notify, i18n);
                }
                let export = egui_widgets::shadcn::Button::new(i18n.t(I18nKey::UserPatchExportMod));
                if ui.add(export).on_hover_text(i18n.t(I18nKey::UserPatchExportModHover)).clicked() {
                    self.mod_export.open = true;
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let output = egui::RichText::new(self.place.output.trim()).weak();
                    ui.add(egui::Label::new(output).truncate()).on_hover_text(i18n.t(I18nKey::UserPatchOutputHover));
                });
            });
        });
    }

    /// `Export as a mod`'s window, if open; `Export` writes it.
    fn mod_export_window(&mut self, ctx: &egui::Context, i18n: &I18nMap, notify: &mut Notification) {
        let outside: Vec<&str> = self.workspace.outside_nemesis().collect();
        if self.mod_export.window(ctx, i18n, notify, &outside) {
            self.export_user_patch_as_mod(notify, i18n);
        }
    }
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
    fn window(&mut self, ctx: &egui::Context, i18n: &I18nMap, notify: &mut Notification, outside: &[&str]) -> bool {
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
