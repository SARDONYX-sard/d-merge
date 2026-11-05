//! Drawing of the hkanno page: tabs, editor, problems. The logic is in the modules next to this
//! directory.

mod preview;
mod toolbar;

use egui_code_editor::{CodeEditor, CodeEditorOutput, EditorCommand, EditorFeatures, Keymap, text_pos::LinePos};
use egui_widgets::shadcn::Button;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::EditorMode,
};
use hkanno_lsp::features::code_action::code_action;

use crate::{
    HkannoPage, HkannoPageCtx,
    language::HkannoLanguage,
    split_dropped,
    tab::{self, HkannoTab, replace_columns},
};

impl HkannoPage {
    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, mut ctx: HkannoPageCtx<'_>) {
        self.handle_drop(ui.ctx(), &mut ctx);
        self.handle_shortcuts(ui.ctx(), &mut ctx);

        // egui's default scroll bars are thin until hovered, which hides where in the file
        // you are. Always-visible solid bars, like a code editor (only on this page).
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();

        // ── Fixed bars (registered before the central area) ────────────────────
        let frame = ctx.panel_frame;
        egui::Panel::top("hkanno_tabs").frame(frame).show(ui, |ui| self.tab_bar(ui, &mut ctx));
        if !self.tabs.is_empty() {
            egui::Panel::top("hkanno_file_bar").frame(frame).show(ui, |ui| self.file_bar(ui, &ctx));
            egui::Panel::top("hkanno_toolbar").frame(frame).show(ui, |ui| self.toolbar(ui, &mut ctx));
            self.problems(ui, &ctx);
            if ctx.settings.show_preview {
                self.preview(ui, &ctx);
            }
        }

        // ── Editor ─────────────────────────────────────────────────────────────
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            if self.tabs.is_empty() {
                ui.centered_and_justified(|ui| ui.weak(ctx.i18n.t(I18nKey::HkannoEmptyMessage)));
            } else {
                self.editor(ui, &mut ctx);
            }
        });

        self.dialogs(ui.ctx(), &mut ctx);
        drop_overlay(ui.ctx(), ctx.i18n);
    }

    /// `[attack.hkx ✖] [idle.hkx* ✖] ...  [Open]`
    fn tab_bar(&mut self, ui: &mut egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        let mut close = None;
        let mut open = None;
        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal().id_salt("hkanno_tab_scroll").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, tab) in self.tabs.iter().enumerate() {
                        // `*` marks unsaved edits.
                        let title = if tab.dirty { format!("{}*", tab.title()) } else { tab.title().to_string() };
                        let response =
                            ui.add(Button::selectable(i == self.active, title)).on_hover_text(&tab.settings.input_path);
                        if response.clicked() {
                            self.active = i;
                        }
                        response.context_menu(|ui| {
                            if tab_menu(ui, tab, ctx.i18n) {
                                close = Some(i);
                            }
                        });
                        if ui.small_button("✖").clicked() {
                            close = Some(i);
                        }
                        ui.add_space(6.0);
                    }
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(Button::new(ctx.i18n.t(I18nKey::HkannoOpenButton)))
                    .on_hover_text(ctx.i18n.t(I18nKey::HkannoOpenHover))
                    .clicked()
                {
                    open =
                        gui_shared::rfd::FileDialog::new().add_filter("Havok Animation", &["hkx", "xml"]).pick_files();
                }
            });
        });

        if let Some(files) = open {
            self.open(files.into_iter().map(|p| p.display().to_string()), ctx);
        }
        if let Some(index) = close {
            self.close(index);
        }
    }

    /// The text editor of the active tab (+ the keymap status line, e.g., `-- INSERT --`).
    fn editor(&mut self, ui: &mut egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        ui.label(egui::RichText::new(ctx.i18n.t(I18nKey::HkannoAnnotationLabel)).weak());

        let lsp = &ctx.lsp_settings;
        let language = HkannoLanguage { semantic: lsp.semantic_tokens, code_action: lsp.code_action };
        let features = EditorFeatures {
            completion: lsp.completion,
            hover: lsp.hover,
            diagnostics: lsp.diagnostics,
            format: lsp.formatter,
            inlay_hints: lsp.inlay_hints,
            signature_help: lsp.signature_help,
        };
        let keymap = to_keymap(ctx.lsp_settings.editor_mode);
        let show_preview = ctx.settings.show_preview;
        let Some(tab) = self.tabs.get_mut(self.active) else { return };

        // Restore the saved cursor once, when the tab is first shown.
        if std::mem::take(&mut tab.restore_cursor) {
            egui_code_editor::set_cursor(ui.ctx(), tab.editor_id(), tab.settings.cursor);
        }

        // Status line first (bottom of the area), so it stays visible while the text scrolls.
        if tab.editor.keymap.status(keymap).is_some() {
            egui::Panel::bottom("hkanno_keymap_status").show(ui, |ui| {
                egui_code_editor::status_line(ui, &tab.editor, keymap);
            });
        }

        let CodeEditorOutput { changed, cursor, command, .. } = egui::ScrollArea::both()
            .id_salt("hkanno_editor_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                CodeEditor::new(tab.settings.input_path.as_str(), &mut tab.text)
                    .language(&language)
                    .diagnostics(&tab.editor_diagnostics)
                    .features(features)
                    .keymap(keymap)
                    .labels(gui_shared::i18n::editor_labels(ctx.i18n))
                    .show(ui, &mut tab.editor)
            })
            .inner;

        if changed {
            tab.dirty = true;
            tab.text_changed();
        }
        if let Some(cursor) = cursor {
            tab.settings.cursor = cursor.to_char_index(&tab.text);
            if show_preview {
                tab.preview.jump_from_editor_line(cursor.line);
            }
        }
        if command == Some(EditorCommand::Save) {
            self.save_active(ctx); // e.g., Vim `:w`
        }
    }

    /// Right pane: XML preview of the active tab.
    fn preview(&mut self, ui: &mut egui::Ui, ctx: &HkannoPageCtx<'_>) {
        let title = ctx.i18n.t(I18nKey::HkannoPreviewTitle);
        let error_title = ctx.i18n.t(I18nKey::HkannoPreviewError);
        let Some(tab) = self.tabs.get_mut(self.active) else { return };

        // Split borrows: the closure reads the document while `preview` is mutated.
        let HkannoTab { preview, text, original, bytes, settings, .. } = tab;
        preview.poll(ui.ctx(), || tab::document(original, text).into_static(), bytes, &settings.input_path, text);

        egui::Panel::right("hkanno_preview").resizable(true).default_size(520.0).show(ui, |ui| {
            match &preview.error {
                Some(err) => ui.colored_label(ui.visuals().error_fg_color, format!("{error_title}: {err}")),
                None => ui.weak(title),
            };
            preview.ui(ui);
        });
    }

    /// Bottom pane: diagnostics of the active tab with quick fixes.
    fn problems(&mut self, ui: &mut egui::Ui, ctx: &HkannoPageCtx<'_>) {
        let (show_diagnostics, show_fixes) = (ctx.lsp_settings.diagnostics, ctx.lsp_settings.code_action);
        let title = ctx.i18n.t(I18nKey::HkannoProblemsLabel);
        let Some(tab) = self.tabs.get_mut(self.active) else { return };
        if !show_diagnostics || tab.diagnostics.is_empty() {
            return;
        }

        let id = tab.editor_id();
        let mut fix = None;

        egui::Panel::bottom("hkanno_problems").resizable(true).default_size(120.0).show(ui, |ui| {
            ui.weak(format!("{title} ({})", tab.diagnostics.len()));
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                // Both lists are built from the same source, in the same order.
                for (lsp_diag, diagnostic) in tab.diagnostics.iter().zip(&tab.editor_diagnostics) {
                    ui.horizontal(|ui| {
                        let severity = diagnostic.severity;
                        ui.colored_label(severity.color(ui.visuals()), severity.icon());

                        // e.g., `Ln 5, Col 1: Missing z value in animmotion.` -> click jumps there.
                        let pos = LinePos { line: diagnostic.line, column: diagnostic.columns.start };
                        let location = format!("Ln {}, Col {}: {}", pos.line + 1, pos.column + 1, diagnostic.message);
                        if ui.link(location).clicked() {
                            egui_code_editor::set_cursor(ui.ctx(), id, pos.to_char_index(&tab.text));
                        }

                        if show_fixes
                            && let Some(action) = code_action(&tab.text, lsp_diag)
                            && ui.add(Button::new(&action.title).small()).clicked()
                        {
                            fix = Some(action.edit);
                        }
                    });
                }
            });
        });

        if let Some(edit) = fix {
            replace_columns(&mut tab.text, edit.range.line, edit.range.span.start..edit.range.span.end, &edit.new_text);
            tab.dirty = true;
            tab.text_changed();
        }
    }

    /// Opens files dropped onto the window (only while this page is shown).
    /// A `.txt` is imported into the active tab instead (see `import_txt`).
    fn handle_drop(&mut self, egui_ctx: &egui::Context, ctx: &mut HkannoPageCtx<'_>) {
        let dropped = egui_ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        let (txt, files) = split_dropped(dropped);
        if !files.is_empty() {
            self.open(files.iter().map(|p| p.display().to_string()), ctx);
        }
        if let Some(path) = txt.last() {
            self.import_txt(path, ctx);
        }
    }
}

/// Right-click menu of a tab. Returns `true` for "Close".
///
/// ```text
/// Open Input Directory
/// Open Output Directory
/// Copy Input Path
/// Copy Output Path
/// ───────────
/// Close
/// ```
fn tab_menu(ui: &mut egui::Ui, tab: &HkannoTab, i18n: &I18nMap) -> bool {
    // The parent: an existing file itself would be opened with its default app.
    let open_dir = |path: &str| {
        let path = std::path::Path::new(path);
        let dir = path.parent().unwrap_or(path);
        if let Err(err) = gui_shared::fs::open_existing_dir_or_ancestor(dir) {
            tracing::error!("{err}");
        }
    };
    let input = tab.settings.input_path.as_str();
    let output = tab.settings.output_path.as_str();

    if ui.button(i18n.t(I18nKey::HkannoTabOpenInputDir)).clicked() {
        open_dir(input);
    }
    if ui.button(i18n.t(I18nKey::HkannoTabOpenOutputDir)).clicked() {
        open_dir(output);
    }
    if ui.button(i18n.t(I18nKey::HkannoTabCopyInputPath)).clicked() {
        ui.ctx().copy_text(input.to_string());
    }
    if ui.button(i18n.t(I18nKey::HkannoTabCopyOutputPath)).clicked() {
        ui.ctx().copy_text(output.to_string());
    }
    ui.separator();
    ui.button(i18n.t(I18nKey::HkannoTabClose)).clicked()
}

/// Settings enum -> editor enum.
const fn to_keymap(mode: EditorMode) -> Keymap {
    match mode {
        EditorMode::Normal => Keymap::Normal,
        EditorMode::Vim => Keymap::Vim,
    }
}

/// Dashed overlay while files are dragged over the window.
fn drop_overlay(ctx: &egui::Context, i18n: &I18nMap) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("hkanno_drop"));
    let painter = ctx.layer_painter(layer);
    let rect = ctx.content_rect();
    let blue = egui::Color32::from_rgb(66, 165, 245);
    painter.rect_filled(rect, 0.0, blue.gamma_multiply(0.15));
    painter.rect_stroke(rect.shrink(4.0), 8.0, egui::Stroke::new(3.0_f32, blue), egui::StrokeKind::Inside);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        i18n.t(I18nKey::HkannoDropHint),
        egui::FontId::proportional(24.0),
        blue,
    );
}
