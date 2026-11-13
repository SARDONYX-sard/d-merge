//! Bars above the editor and the page's dialogs.
//!
//! ```text
//! Output: [attack.modified.hkx                      ]  Format: [HKX SE (64-bit) ▼]        <- file bar
//! [Save] [Revert] [Format] [⚙]  Keymap: [Vim ▼]                [Hide Preview]  <- toolbar
//! ```

use egui_widgets::shadcn::{Button, Checkbox, ComboBox, TextEdit};
use gui_shared::{i18n::I18nKey, settings::pages::convert::OutputFormat};

use crate::{HkannoPage, HkannoPageCtx};

impl HkannoPage {
    /// `Output: [path]  Format: [HKX SE (64-bit) ▼]`
    pub(crate) fn file_bar(&mut self, ui: &mut egui::Ui, ctx: &HkannoPageCtx<'_>) {
        let Some(tab) = self.active_tab() else { return };
        ui.horizontal(|ui| {
            ui.label(ctx.i18n.t(I18nKey::OutputLabel));

            // Right-to-left: the format box takes its width first, the path gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut index = OutputFormat::ALL.iter().position(|f| *f == tab.settings.format).unwrap_or(0);
                if ComboBox::from_id_salt("hkanno_out_format")
                    .width(150.0) // fits "HKX SE (64-bit)"
                    .show_index(ui, &mut index, OutputFormat::ALL.len(), |i| OutputFormat::ALL[i].label())
                    .changed()
                {
                    tab.set_format(OutputFormat::ALL[index]); // also fixes the output extension
                }
                ui.label(ctx.i18n.t(I18nKey::OutputFormatLabel));

                let width = ui.available_width();
                ui.add(TextEdit::singleline(&mut tab.settings.output_path).desired_width(width));
            });
        });
    }

    /// `[Save] [Revert] [Format] [⚙]  Keymap: [Vim ▼]   [Hide Preview]`
    pub(crate) fn toolbar(&mut self, ui: &mut egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        let i18n = ctx.i18n;
        ui.horizontal(|ui| {
            if ui.add(Button::new(i18n.t(I18nKey::HkannoSaveButton)).primary()).clicked() {
                self.save_active(ctx);
            }

            let dirty = self.tabs.get(self.active).is_some_and(|tab| tab.dirty);
            if ui.add_enabled(dirty, Button::new(i18n.t(I18nKey::RevertButton))).clicked() {
                self.show_revert = true;
            }

            if ctx.lsp_settings.formatter
                && ui.add(Button::new(i18n.t(I18nKey::HkannoFormatButton))).clicked()
                && let Some(tab) = self.active_tab()
            {
                let formatted = hkanno_lsp::features::format::format_document(&tab.text);
                if formatted != tab.text {
                    tab.text = formatted;
                    tab.dirty = true;
                    tab.text_changed();
                }
            }

            let has_tab = !self.tabs.is_empty();
            let import = Button::new(i18n.t(I18nKey::HkannoImportTxt));
            if ui.add_enabled(has_tab, import).on_hover_text(i18n.t(I18nKey::HkannoImportTxtHover)).clicked()
                && let Some(path) = gui_shared::rfd::FileDialog::new().add_filter("Text", &["txt"]).pick_file()
            {
                self.import_txt(&path, ctx);
            }
            let export = Button::new(i18n.t(I18nKey::HkannoExportTxt));
            if ui.add_enabled(has_tab, export).on_hover_text(i18n.t(I18nKey::HkannoExportTxtHover)).clicked() {
                self.export_txt(ctx);
            }

            // The keymap and the language features are in the editor's status bar, as in VS Code.

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let show = &mut ctx.settings.show_preview;
                let label = if *show { I18nKey::EditorHidePreview } else { I18nKey::EditorShowPreview };
                if ui.add(Button::new(i18n.t(label))).clicked() {
                    *show = !*show;
                }
            });
        });
    }

    /// Asks where to save, then writes the active tab's annotation text as `.txt`.
    fn export_txt(&mut self, ctx: &mut HkannoPageCtx<'_>) {
        let Some(tab) = self.active_tab() else { return };
        let default = tab.default_txt_path();
        let mut dialog = gui_shared::rfd::FileDialog::new().add_filter("Text", &["txt"]);
        if let Some(dir) = default.parent() {
            dialog = dialog.set_directory(dir);
        }
        if let Some(name) = default.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy());
        }
        let Some(path) = dialog.save_file() else { return };
        match tab.export_txt(&path) {
            Ok(()) => ctx.notify.success(format!("{}: {}", ctx.i18n.t(I18nKey::ExportedMessage), path.display())),
            Err(err) => {
                let msg = format!("{}: {err}", ctx.i18n.t(I18nKey::HkannoSaveFailed));
                tracing::error!("{msg}");
                ctx.notify.error(msg);
            }
        }
    }

    /// `Ctrl+S` (key bindings) saves, in every keymap; Vim also has `:w`.
    pub(crate) fn handle_shortcuts(&mut self, ctx: &mut HkannoPageCtx<'_>) {
        if self.key_command(keybinding::commands::EDITOR_SAVE) {
            self.save_active(ctx);
        }
    }

    /// Revert confirmation and the language feature options window.
    pub(crate) fn dialogs(&mut self, egui_ctx: &egui::Context, ctx: &mut HkannoPageCtx<'_>) {
        if self.show_revert {
            self.revert_modal(egui_ctx, ctx);
        }
        if self.show_lsp_options {
            lsp_options_window(egui_ctx, ctx, &mut self.show_lsp_options);
        }
    }

    /// "Discard the edits?" with the text that will come back.
    fn revert_modal(&mut self, egui_ctx: &egui::Context, ctx: &HkannoPageCtx<'_>) {
        let i18n = ctx.i18n;
        let Some(tab) = self.tabs.get_mut(self.active) else {
            self.show_revert = false;
            return;
        };

        let modal = egui::Modal::new(egui::Id::unique("hkanno_revert")).show(egui_ctx, |ui| {
            ui.set_max_width(640.0);
            ui.label(i18n.t(I18nKey::HkannoRevertConfirm));
            ui.add_space(4.0);

            // e.g., the original `# numOriginalFrames: 38\ntrackName: ...` (read-only).
            let original = tab.original.to_string();
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(original).monospace()).extend());
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(Button::new(i18n.t(I18nKey::HkannoApplyButton)).destructive()).clicked() {
                    tab.revert();
                    ui.close();
                }
                if ui.add(Button::new(i18n.t(I18nKey::CancelButton))).clicked() {
                    ui.close();
                }
            });
        });
        if modal.should_close() {
            self.show_revert = false;
        }
    }
}

/// Window with one checkbox per language feature.
fn lsp_options_window(egui_ctx: &egui::Context, ctx: &mut HkannoPageCtx<'_>, open: &mut bool) {
    egui_widgets::window_near_pointer(egui_ctx, ctx.i18n.t(I18nKey::LspOptionsTitle))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .show(egui_ctx, |ui| {
            let options = [
                (&mut ctx.lsp_settings.completion, I18nKey::LspCompletion, I18nKey::LspCompletionHover),
                (&mut ctx.lsp_settings.code_action, I18nKey::LspCodeAction, I18nKey::LspCodeActionHover),
                (&mut ctx.lsp_settings.diagnostics, I18nKey::LspDiagnostics, I18nKey::LspDiagnosticsHover),
                (&mut ctx.lsp_settings.formatter, I18nKey::LspFormatter, I18nKey::LspFormatterHover),
                (&mut ctx.lsp_settings.semantic_tokens, I18nKey::LspSemanticTokens, I18nKey::LspSemanticTokensHover),
                (&mut ctx.lsp_settings.hover, I18nKey::LspHover, I18nKey::LspHoverHover),
                (&mut ctx.lsp_settings.inlay_hints, I18nKey::LspInlayHints, I18nKey::LspInlayHintsHover),
                (&mut ctx.lsp_settings.signature_help, I18nKey::LspSignatureHelp, I18nKey::LspSignatureHelpHover),
            ];
            for (value, label, hover) in options {
                ui.add(Checkbox::new(value, ctx.i18n.t(label))).on_hover_text(ctx.i18n.t(hover));
            }
        });
}
