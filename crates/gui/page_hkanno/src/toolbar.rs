//! Bars above the editor and the page's dialogs.
//!
//! ```text
//! Output: [attack.modified.hkx                      ]  Format: [Amd64 ▼]        <- file bar
//! [Save] [Revert] [Format] [⚙]  Keymap: [Vim ▼]                [Hide Preview]  <- toolbar
//! ```

use egui_widgets::shadcn::{Button, Checkbox, ComboBox, TextEdit};
use gui_shared::{
    i18n::I18nKey,
    settings::{convert::OutFormat, hkanno::EditorMode},
};

use crate::{HkannoPage, HkannoPageCtx};

impl HkannoPage {
    /// `Output: [path]  Format: [Amd64 ▼]`
    pub(crate) fn file_bar(&mut self, ui: &mut egui::Ui, ctx: &HkannoPageCtx<'_>) {
        let Some(tab) = self.active_tab() else { return };
        ui.horizontal(|ui| {
            ui.label(ctx.i18n.t(I18nKey::ConvertOutputLabel));

            // Right-to-left: the format box takes its width first, the path gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut index =
                    OutFormat::ALL.iter().position(|f| *f == tab.settings.format).unwrap_or(0);
                if ComboBox::from_id_salt("hkanno_out_format")
                    .width(100.0)
                    .show_index(ui, &mut index, OutFormat::ALL.len(), |i| OutFormat::ALL[i].label())
                    .changed()
                {
                    tab.set_format(OutFormat::ALL[index]); // also fixes the output extension
                }
                ui.label(ctx.i18n.t(I18nKey::ConvertOutputFormatLabel));

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
            if ui.add_enabled(dirty, Button::new(i18n.t(I18nKey::HkannoRevertButton))).clicked() {
                self.show_revert = true;
            }

            if ctx.settings.lsp.formatter
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

            if ui
                .add(Button::selectable(self.show_lsp_options, "⚙"))
                .on_hover_text(i18n.t(I18nKey::HkannoLspOptionsTitle))
                .clicked()
            {
                self.show_lsp_options = !self.show_lsp_options;
            }

            ui.separator();
            editor_mode_select(ui, ctx);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let show = &mut ctx.settings.show_preview;
                let label =
                    if *show { I18nKey::HkannoHidePreview } else { I18nKey::HkannoShowPreview };
                if ui.add(Button::new(i18n.t(label))).clicked() {
                    *show = !*show;
                }
            });
        });
    }

    /// Saves the active tab and reports the result.
    pub(crate) fn save_active(&mut self, ctx: &mut HkannoPageCtx<'_>) {
        let Some(tab) = self.active_tab() else { return };
        match tab.save() {
            Ok(()) => ctx.notify.success(format!(
                "{}: {}",
                ctx.i18n.t(I18nKey::HkannoSaved),
                tab.settings.output_path
            )),
            Err(err) => {
                let msg = format!("{}: {err}", ctx.i18n.t(I18nKey::HkannoSaveFailed));
                tracing::error!("{msg}");
                ctx.notify.error(msg);
            }
        }
    }

    /// `Ctrl+S` saves (works in every keymap; Vim also has `:w`).
    pub(crate) fn handle_shortcuts(
        &mut self,
        egui_ctx: &egui::Context,
        ctx: &mut HkannoPageCtx<'_>,
    ) {
        if egui_ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
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

/// `Keymap: [Normal ▼]`: plain editing or Vim.
fn editor_mode_select(ui: &mut egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
    ui.label(ctx.i18n.t(I18nKey::HkannoEditorMode))
        .on_hover_text(ctx.i18n.t(I18nKey::HkannoEditorModeHover));

    let mode = &mut ctx.settings.editor_mode;
    let mut index = EditorMode::ALL.iter().position(|m| m == mode).unwrap_or(0);
    if ComboBox::from_id_salt("hkanno_editor_mode")
        .width(90.0)
        .show_index(ui, &mut index, EditorMode::ALL.len(), |i| EditorMode::ALL[i].label())
        .changed()
    {
        *mode = EditorMode::ALL[index];
    }
}

/// Window with one checkbox per language feature.
fn lsp_options_window(egui_ctx: &egui::Context, ctx: &mut HkannoPageCtx<'_>, open: &mut bool) {
    let i18n = ctx.i18n;
    let lsp = &mut ctx.settings.lsp;
    egui::Window::new(i18n.t(I18nKey::HkannoLspOptionsTitle))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .show(egui_ctx, |ui| {
            let options = [
                (
                    &mut lsp.completion,
                    I18nKey::HkannoLspCompletion,
                    I18nKey::HkannoLspCompletionHover,
                ),
                (
                    &mut lsp.code_action,
                    I18nKey::HkannoLspCodeAction,
                    I18nKey::HkannoLspCodeActionHover,
                ),
                (
                    &mut lsp.diagnostics,
                    I18nKey::HkannoLspDiagnostics,
                    I18nKey::HkannoLspDiagnosticsHover,
                ),
                (&mut lsp.formatter, I18nKey::HkannoLspFormatter, I18nKey::HkannoLspFormatterHover),
                (
                    &mut lsp.semantic_tokens,
                    I18nKey::HkannoLspSemanticTokens,
                    I18nKey::HkannoLspSemanticTokensHover,
                ),
                (&mut lsp.hover, I18nKey::HkannoLspHover, I18nKey::HkannoLspHoverHover),
                (
                    &mut lsp.inlay_hints,
                    I18nKey::HkannoLspInlayHints,
                    I18nKey::HkannoLspInlayHintsHover,
                ),
                (
                    &mut lsp.signature_help,
                    I18nKey::HkannoLspSignatureHelp,
                    I18nKey::HkannoLspSignatureHelpHover,
                ),
            ];
            for (value, label, hover) in options {
                ui.add(Checkbox::new(value, i18n.t(label))).on_hover_text(i18n.t(hover));
            }
        });
}
