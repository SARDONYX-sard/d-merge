//! The right side: the XML editor of the open object, and its patch preview below.

use egui_code_editor::{CodeEditor, EditorAction, EditorFeatures, Keymap, Navigation};
use egui_widgets::shadcn::{Button, Checkbox};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::{EditorMode, LspOptions},
};

use super::{preview::PreviewEvent, references::references_window};
use crate::patch::{EditorResult, FocusAt, PatchTool, language::NemesisXml};

/// The id of the palette's "focus the object tree" command.
const ACTION_FOCUS_TREE: &str = "focus_tree";

impl PatchTool {
    pub(in crate::patch) fn object_editor(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let Some(id) = self.selected.clone() else {
            egui::CentralPanel::default().show(ui, |ui| ui.weak(i18n.t(I18nKey::NemesisSelectObject)));
            return;
        };
        self.open_in_editor(&id);
        let editor_id = egui_code_editor::editor_id(("nemesis_object", &id));
        self.apply_focus_request(ui.ctx(), editor_id);
        self.refresh_preview(&id);

        // No preview for a new object: it is written as is.
        let is_new = self.workspace.behaviors[self.active].added.contains_key(&id);
        let preview_event = if is_new { None } else { self.preview_panel(ui, i18n) };
        let result = self.editor_panel(ui, i18n, &id, is_new);

        if let Some(view) = &mut self.preview_view {
            view.follow_editor_line(self.cursor_line, &self.editor_text);
        }
        if self.apply_editor_result(result, &id) {
            self.enter_tree(ui, editor_id);
        }
        if let Some((document, line)) = references_window(ui.ctx(), &mut self.references) {
            if document != id {
                self.history.visit(&id);
                document.clone_into(self.selected.get_or_insert_default());
            }
            self.focus_editor = Some(FocusAt::Line(line));
        }
        match preview_event {
            Some(PreviewEvent::Revert(block)) => self.revert_block(&id, block),
            Some(PreviewEvent::JumpToEditor(row)) => {
                let line = self.preview_view.as_ref().and_then(|view| view.editor_line_of(row, &self.editor_text));
                self.focus_editor = line.map(FocusAt::Line);
            }
            None => {}
        }
    }

    /// `Patch preview ▲ ▼ 2/5 ✔` and its lines.
    fn preview_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) -> Option<PreviewEvent> {
        let mut event = None;
        let panel = if self.preview_side {
            egui::Panel::right("nemesis_preview_side").default_size(520.0)
        } else {
            egui::Panel::bottom("nemesis_preview").default_size(260.0)
        };
        panel.resizable(true).show(ui, |ui| {
            let Some((patch, check)) = &self.preview else { return };
            ui.horizontal(|ui| {
                ui.strong(i18n.t(I18nKey::NemesisPatchPreview));
                if let Some(view) = &mut self.preview_view {
                    super::preview::nav_ui(view, ui, i18n.t(I18nKey::NemesisJumpDiffHover));
                }
                match check {
                    _ if patch.is_none() => {
                        ui.weak(i18n.t(I18nKey::NemesisNoChange));
                    }
                    Ok(()) => {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 120), "✔")
                            .on_hover_text(i18n.t(I18nKey::NemesisParserOk));
                    }
                    Err(err) => {
                        ui.colored_label(ui.visuals().error_fg_color, format!("✖ {err}"));
                    }
                }
            });
            if let Some(view) = &mut self.preview_view {
                event = super::preview::lines_ui(view, ui, i18n.t(I18nKey::NemesisRevertHunkHover));
            }
        });
        event
    }

    /// The toolbar, the editor and the Vim status line. Returns what the editor asked for: it is
    /// done afterwards, as the language still borrows the behavior here.
    fn editor_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap, id: &str, is_new: bool) -> EditorResult {
        let names = self.names();
        let values = self.field_values();
        // The class (and fields) around the cursor line of last frame: field names / hover.
        let context = nemesis_diff::schema::field_context(&self.editor_text, self.cursor_line);
        let language = NemesisXml {
            events: &names.events,
            variables: &names.variables,
            variable_types: &names.variable_types,
            values: &values,
            graph: &names.graph,
            fields: context.as_ref().map(|(class, fields)| (class.as_str(), fields.as_slice())),
            behavior: &self.workspace.behaviors[self.active],
        };
        let features = editor_features(&self.lsp);
        let keymap = match self.editor_mode {
            EditorMode::Normal => Keymap::Normal,
            EditorMode::Vim => Keymap::Vim,
        };
        let labels = gui_shared::i18n::editor_labels(i18n);
        let actions = [EditorAction { id: ACTION_FOCUS_TREE, label: i18n.t(I18nKey::NemesisFocusTree) }];

        let mut result = EditorResult::default();
        egui::CentralPanel::default().show(ui, |ui| {
            // A new object has no preview panel: the parser's verdict goes here.
            let error = self.preview.as_ref().and_then(|(_, check)| check.as_ref().err()).filter(|_| is_new);
            toolbar(ui, i18n, &mut self.editor_mode, &mut self.show_lsp_options, error.map(String::as_str));
            ui.horizontal(|ui| diff_view_options(ui, i18n, &mut self.show_inline_diff, &mut self.preview_side));
            if self.show_lsp_options {
                lsp_options_window(ui.ctx(), i18n, &mut self.lsp, &mut self.show_lsp_options);
            }
            if self.editor.keymap.status(keymap).is_some() {
                egui::Panel::bottom("nemesis_keymap_status")
                    .show(ui, |ui| egui_code_editor::status_line(ui, &self.editor, keymap));
            }
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let output = CodeEditor::new(("nemesis_object", id), &mut self.editor_text)
                    .language(&language)
                    .diagnostics(&self.field_problems)
                    .features(features)
                    .keymap(keymap)
                    .labels(labels)
                    .actions(&actions)
                    .inline_diff(self.show_inline_diff.then_some(&self.inline_diff))
                    .show(ui, &mut self.editor);
                if let Some(cursor) = output.cursor {
                    self.cursor_line = cursor.line;
                }
                result.changed = output.changed;
                result.command = output.command;
                result.focus_tree = output.action == Some(ACTION_FOCUS_TREE);
                // Kept past the borrows of this frame, so owned here.
                match output.navigation {
                    Some(Navigation::Definition(location)) => {
                        result.definition = Some((location.document.into_owned(), location.line));
                    }
                    Some(Navigation::References(references)) => self.references = Some(references.into()),
                    None => {}
                }
            });
        });
        result
    }
}

/// `☑ Inline diff  ☑ Preview on the side`
fn diff_view_options(ui: &mut egui::Ui, i18n: &I18nMap, inline_diff: &mut bool, preview_side: &mut bool) {
    ui.add(Checkbox::new(inline_diff, i18n.t(I18nKey::NemesisInlineDiff)))
        .on_hover_text(i18n.t(I18nKey::NemesisInlineDiffHover));
    ui.add(Checkbox::new(preview_side, i18n.t(I18nKey::NemesisPreviewSide)));
}

/// The language features the settings turn on, and what XML has no use for.
const fn editor_features(lsp: &LspOptions) -> EditorFeatures {
    EditorFeatures {
        completion: lsp.completion,
        hover: lsp.hover,
        diagnostics: lsp.diagnostics,
        format: false,
        inlay_hints: false,
        signature_help: lsp.signature_help,
    }
}

/// `Keys: [Vim ▼] ⚙  ✖ error`. `⚙` has the same language switches as the hkanno editor's.
fn toolbar(ui: &mut egui::Ui, i18n: &I18nMap, mode: &mut EditorMode, show_options: &mut bool, error: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(i18n.t(I18nKey::LspEditorMode));
        let mut index = EditorMode::ALL.iter().position(|m| m == mode).unwrap_or(0);
        if egui::ComboBox::from_id_salt("nemesis_editor_mode")
            .width(80.0)
            .show_index(ui, &mut index, EditorMode::ALL.len(), |i| EditorMode::ALL[i].label())
            .changed()
        {
            *mode = EditorMode::ALL[index];
        }
        if ui
            .add(Button::selectable(*show_options, "⚙"))
            .on_hover_text(i18n.t(I18nKey::HkannoLspOptionsTitle))
            .clicked()
        {
            *show_options = !*show_options;
        }
        if let Some(error) = error {
            ui.colored_label(ui.visuals().error_fg_color, format!("✖ {error}"));
        }
    });
}

/// The language features that apply to the object editor (the hkanno options window's subset).
fn lsp_options_window(ctx: &egui::Context, i18n: &I18nMap, lsp: &mut LspOptions, open: &mut bool) {
    egui::Window::new(i18n.t(I18nKey::HkannoLspOptionsTitle)).open(open).resizable(false).collapsible(false).show(
        ctx,
        |ui| {
            let options = [
                (&mut lsp.completion, I18nKey::LspCompletion, I18nKey::LspCompletionHover),
                (&mut lsp.diagnostics, I18nKey::LspDiagnostics, I18nKey::LspDiagnosticsHover),
                (&mut lsp.hover, I18nKey::LspHover, I18nKey::LspHoverHover),
                (&mut lsp.signature_help, I18nKey::LspSignatureHelp, I18nKey::LspSignatureHelpHover),
            ];
            for (value, label, hover) in options {
                ui.add(Checkbox::new(value, i18n.t(label))).on_hover_text(i18n.t(hover));
            }
        },
    );
}
