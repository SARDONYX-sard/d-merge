//! Bars and dialogs around the editor:
//! - file bar: output path + output format of the active tab
//! - toolbar: Save / Revert / Format / language feature options / preview toggle
//! - dialogs: revert confirmation, language feature options
//! - shortcuts: `Ctrl+S` save, `Shift+Alt+F` format

use d_merge_gui_shared::{i18n::I18nKey, settings::convert::OutFormat};

use crate::{
    app::App,
    ui::shadcn::{Button, Checkbox, ComboBox, TextEdit},
};

impl App {
    /// `Output: [path            ] Format: [Amd64 ▼]`
    pub(super) fn ui_hkanno_file_bar(&mut self, ui: &mut egui::Ui) {
        let output_label = self.i18n.t(I18nKey::ConvertOutputLabel);
        let format_label = self.i18n.t(I18nKey::ConvertOutputFormatLabel);
        let Some(tab) = self.hkanno.active_tab() else { return };

        ui.horizontal(|ui| {
            ui.label(output_label);

            // Right-to-left: the format box takes its width first, the path gets the rest.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut index =
                    OutFormat::ALL.iter().position(|f| *f == tab.settings.format).unwrap_or(0);
                if ComboBox::from_id_salt("hkanno_out_format")
                    .width(110.0)
                    .show_index(ui, &mut index, OutFormat::ALL.len(), |i| OutFormat::ALL[i].label())
                    .changed()
                {
                    tab.set_format(OutFormat::ALL[index]);
                }
                ui.label(format_label);

                let width = ui.available_width();
                ui.add(TextEdit::singleline(&mut tab.settings.output_path).desired_width(width));
            });
        });
    }

    /// `[Save] [Revert] [Format] [⚙] ........ [Hide Preview]`
    pub(super) fn ui_hkanno_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.add(Button::new(self.i18n.t(I18nKey::HkannoSaveButton)).primary()).clicked() {
                self.save_active_hkanno();
            }
            if ui.add(Button::new(self.i18n.t(I18nKey::HkannoRevertButton))).clicked() {
                self.hkanno.show_revert = true;
            }
            if self.settings.hkanno.lsp.formatter
                && ui
                    .add(Button::new(self.i18n.t(I18nKey::HkannoFormatButton)))
                    .on_hover_text("Shift+Alt+F")
                    .clicked()
            {
                self.format_active_hkanno();
            }
            if ui
                .add(Button::selectable(self.hkanno.show_lsp_options, "⚙"))
                .on_hover_text(self.i18n.t(I18nKey::HkannoLspOptionsTitle))
                .clicked()
            {
                self.hkanno.show_lsp_options ^= true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let show = &mut self.settings.hkanno.show_preview;
                let label =
                    if *show { I18nKey::HkannoHidePreview } else { I18nKey::HkannoShowPreview };
                if ui.add(Button::new(self.i18n.t(label))).clicked() {
                    *show ^= true;
                }
            });
        });
    }

    /// `Ctrl+S` = save, `Shift+Alt+F` = format.
    pub(super) fn handle_hkanno_shortcuts(&mut self, ctx: &egui::Context) {
        if self.hkanno.tabs.is_empty() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_active_hkanno();
        }
        let format_keys = egui::Modifiers::SHIFT | egui::Modifiers::ALT;
        if self.settings.hkanno.lsp.formatter
            && ctx.input_mut(|i| i.consume_key(format_keys, egui::Key::F))
        {
            self.format_active_hkanno();
        }
    }

    fn save_active_hkanno(&mut self) {
        let Some(tab) = self.hkanno.active_tab() else { return };
        match tab.save() {
            Ok(()) => self.notify.success(self.i18n.t(I18nKey::HkannoSaved)),
            Err(err) => {
                tracing::error!("{err}");
                self.notify.error(format!("{}: {err}", self.i18n.t(I18nKey::HkannoSaveFailed)));
            }
        }
    }

    fn format_active_hkanno(&mut self) {
        let Some(tab) = self.hkanno.active_tab() else { return };
        let formatted = hkanno_lsp::features::format::format_document(&tab.text);
        if formatted != tab.text {
            tab.text = formatted;
            tab.dirty = true;
            tab.text_changed();
        }
    }

    /// Revert confirmation + language feature options.
    pub(super) fn ui_hkanno_dialogs(&mut self, ctx: &egui::Context) {
        self.ui_hkanno_revert_modal(ctx);
        self.ui_hkanno_lsp_options(ctx);
    }

    /// Shows the original annotations and asks before discarding the edits.
    fn ui_hkanno_revert_modal(&mut self, ctx: &egui::Context) {
        if !self.hkanno.show_revert {
            return;
        }
        let (confirm, apply, cancel) = (
            self.i18n.t(I18nKey::HkannoRevertConfirm),
            self.i18n.t(I18nKey::HkannoApplyButton),
            self.i18n.t(I18nKey::CancelButton),
        );
        let Some(tab) = self.hkanno.tabs.get_mut(self.hkanno.active) else {
            self.hkanno.show_revert = false;
            return;
        };

        let mut close = false;
        let modal = egui::Modal::new(egui::Id::unique("hkanno_revert")).show(ctx, |ui| {
            ui.set_width(ctx.content_rect().width() * 0.6);
            ui.label(confirm);
            ui.separator();

            let mut original = tab.original.to_string();
            egui::ScrollArea::vertical().max_height(ctx.content_rect().height() * 0.6).show(
                ui,
                |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut original)
                            .code_editor()
                            .interactive(false)
                            .desired_width(f32::INFINITY),
                    );
                },
            );

            ui.separator();
            ui.horizontal(|ui| {
                if ui.add(Button::new(apply).primary()).clicked() {
                    tab.revert();
                    close = true;
                }
                if ui.add(Button::new(cancel)).clicked() {
                    close = true;
                }
            });
        });

        if close || modal.should_close() {
            self.hkanno.show_revert = false;
        }
    }

    /// Window with one checkbox per language feature.
    fn ui_hkanno_lsp_options(&mut self, ctx: &egui::Context) {
        let mut open = self.hkanno.show_lsp_options;
        let lsp = &mut self.settings.hkanno.lsp;
        let i18n = &self.i18n;

        egui::Window::new(i18n.t(I18nKey::HkannoLspOptionsTitle))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
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
                    (
                        &mut lsp.formatter,
                        I18nKey::HkannoLspFormatter,
                        I18nKey::HkannoLspFormatterHover,
                    ),
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
                for (value, label, description) in options {
                    ui.add(Checkbox::new(value, i18n.t(label)));
                    ui.weak(i18n.t(description));
                    ui.separator();
                }
            });

        self.hkanno.show_lsp_options = open;
    }
}
