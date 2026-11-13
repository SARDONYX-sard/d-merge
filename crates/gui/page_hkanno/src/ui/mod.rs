//! Drawing of the hkanno page: tabs, editor, problems. The logic is in the modules next to this
//! directory.

mod preview;
mod toolbar;

use egui_code_editor::{
    CodeEditor, CodeEditorOutput, EditorCommand, EditorFeatures, Keymap, ProblemGroup, ProblemsPanel, text_pos::LinePos,
};
use egui_widgets::shadcn::Button;
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::editor::EditorMode,
};
use hkanno_lsp::features::code_action::code_action;
use keybinding::commands;

use crate::{
    HkannoPage, HkannoPageCtx,
    language::HkannoLanguage,
    split_dropped,
    tab::{self, HkannoTab, replace_columns},
};

/// What this page's editor acts on: see [`HkannoPage::apply_command`].
const HOST_COMMANDS: [EditorCommand; 5] = [
    EditorCommand::NextTab,
    EditorCommand::PreviousTab,
    EditorCommand::CloseTab,
    EditorCommand::TogglePreview,
    EditorCommand::RevealFile,
];

impl HkannoPage {
    /// Draws the page.
    pub fn ui(&mut self, ui: &mut egui::Ui, mut ctx: HkannoPageCtx<'_>) {
        self.handle_drop(ui.ctx(), &mut ctx);
        let context = self.key_context(ui.ctx(), &ctx);
        self.key_commands = ctx.keys.dispatch(ui.ctx(), &context);
        self.handle_shortcuts(&mut ctx);
        if self.key_command(commands::FILE_REVEAL) {
            self.reveal_active(&mut ctx);
        }
        if self.key_command(commands::FILE_COPY_PATH)
            && let Some(tab) = self.tabs.get(self.active)
        {
            ui.ctx().copy_text(tab.settings.input_path.clone());
        }

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

    /// `│ attack.hkx ✖ │ idle.hkx ● │ ...  [Open]`: VS Code's tabs ([`egui_widgets::EditorTab`]).
    /// `●`: unsaved edits; red / yellow: the file has errors / warnings.
    fn tab_bar(&mut self, ui: &mut egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        let mut close = None;
        let mut open = None;
        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal().id_salt("hkanno_tab_scroll").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; // the tabs touch, with a line between them
                    for (i, tab) in self.tabs.iter().enumerate() {
                        let (errors, warnings) = egui_code_editor::problems::counts(&tab.editor_diagnostics);
                        let tone = match (errors, warnings) {
                            (0, 0) => None,
                            (0, _) => Some(egui_code_editor::Severity::Warning.color(ui.visuals())),
                            _ => Some(egui_code_editor::Severity::Error.color(ui.visuals())),
                        };
                        let focused = ui.memory(|m| m.has_focus(tab.editor_id()));
                        let shown = egui_widgets::EditorTab::new(tab.title(), i == self.active)
                            .focused(focused)
                            .modified(tab.dirty)
                            .tone(tone)
                            .show(ui);
                        let response = shown.response.on_hover_text(&tab.settings.input_path);
                        if response.clicked() {
                            self.active = i;
                        }
                        response.context_menu(|ui| {
                            if tab_menu(ui, tab, ctx.i18n) {
                                close = Some(i);
                            }
                        });
                        if shown.close {
                            close = Some(i);
                        }
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
        if self.key_command(commands::TABS_NEXT) {
            self.step_tab(ui, true);
        } else if self.key_command(commands::TABS_PREVIOUS) {
            self.step_tab(ui, false);
        }

        let i18n = ctx.i18n;
        let actions = [
            EditorCommand::NextTab.action(i18n.t(I18nKey::EditorNextTab)),
            EditorCommand::PreviousTab.action(i18n.t(I18nKey::EditorPreviousTab)),
            EditorCommand::CloseTab.action(i18n.t(I18nKey::EditorCloseTab)),
            EditorCommand::TogglePreview.action(i18n.t(I18nKey::EditorTogglePreview)),
        ];
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
        let mut toggle_problems = false;
        egui::Panel::bottom("hkanno_status_bar").show(ui, |ui| {
            // Its counts show or hide the Problems panel, as VS Code's.
            toggle_problems = egui_code_editor::status_line(ui, &tab.editor, keymap, &tab.text).clicked();
        });
        if toggle_problems {
            self.problems_hidden = !self.problems_hidden;
        }
        let Some(tab) = self.tabs.get_mut(self.active) else { return };

        let CodeEditorOutput { changed, cursor, command, .. } = egui::ScrollArea::both()
            .id_salt("hkanno_editor_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                CodeEditor::new(tab.settings.input_path.as_str(), &mut tab.text)
                    .language(&language)
                    .diagnostics(&tab.editor_diagnostics)
                    .features(features)
                    .keymap(keymap)
                    .labels(egui_code_editor::EditorLabels::from_texts(gui_shared::i18n::editor_label_texts(i18n)))
                    .actions(&actions)
                    .host_commands(&HOST_COMMANDS)
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
        if let Some(command) = command {
            self.apply_command(command, ui, ctx);
        }
    }

    /// Does what the editor asked for. e.g., Vim `:w`, `<Space>l`, or the palette.
    fn apply_command(&mut self, command: EditorCommand, ui: &egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        match command {
            EditorCommand::Save => self.save_active(ctx),
            EditorCommand::NextTab => self.step_tab(ui, true),
            EditorCommand::PreviousTab => self.step_tab(ui, false),
            EditorCommand::CloseTab => self.close_active_from_keyboard(ui, ctx),
            EditorCommand::TogglePreview => ctx.settings.show_preview = !ctx.settings.show_preview,
            EditorCommand::RevealFile => self.reveal_active(ctx),
            EditorCommand::GoToDefinition
            | EditorCommand::FindReferences
            | EditorCommand::JumpBack
            | EditorCommand::JumpForward
            | EditorCommand::ToggleExplorer
            | EditorCommand::ToggleInlineDiff
            | EditorCommand::NextChange
            | EditorCommand::PreviousChange => {}
        }
    }

    /// The next or previous tab, with the keyboard going along to its editor.
    fn step_tab(&mut self, ui: &egui::Ui, forward: bool) {
        if let Some(next) = crate::neighbor_tab(self.active, self.tabs.len(), forward) {
            self.active = next;
            self.focus_active_editor(ui);
        }
    }

    /// `:q` and the like are quick to press and, unlike `✖`, close without looking at the tab, so
    /// unsaved edits stop them, as in Vim. There is no way to get these edits back.
    fn close_active_from_keyboard(&mut self, ui: &egui::Ui, ctx: &mut HkannoPageCtx<'_>) {
        if self.tabs.get(self.active).is_some_and(|tab| tab.dirty) {
            ctx.notify.error(ctx.i18n.t(I18nKey::HkannoTabUnsaved).to_owned());
            return;
        }
        self.close(self.active);
        self.focus_active_editor(ui);
    }

    fn focus_active_editor(&self, ui: &egui::Ui) {
        if let Some(tab) = self.tabs.get(self.active) {
            let id = tab.editor_id();
            ui.memory_mut(|memory| memory.request_focus(id));
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
        let id = tab.editor_id();
        let cursor = &mut self.problems_cursor;
        // `Ctrl+Shift+L`, as VS Code's `Ctrl+Shift+M`: the panel shows and gets the keyboard; if it
        // has it, it hides and the editor gets it back.
        let toggle = self.key_commands.iter().position(|asked| *asked == commands::PROBLEMS_TOGGLE);
        if let Some(at) = toggle {
            self.key_commands.swap_remove(at);
            if cursor.focused {
                (cursor.focused, self.problems_hidden) = (false, true);
                ui.memory_mut(|m| m.request_focus(id));
            } else if show_diagnostics && !tab.diagnostics.is_empty() {
                (cursor.focused, self.problems_hidden) = (true, false);
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        } else if ui.memory(|m| m.has_focus(id)) {
            cursor.focused = false; // e.g. a click in the editor
        }
        if !show_diagnostics || self.problems_hidden || tab.diagnostics.is_empty() {
            cursor.focused = false;
            return;
        }
        let mut fix = None;

        // One file: its problems without a header, each with its quick fix if it has one.
        let group = ProblemGroup { title: std::borrow::Cow::Borrowed(""), problems: &tab.editor_diagnostics };
        let text = &tab.text;
        let was_focused = cursor.focused;
        let panel = ProblemsPanel::new("hkanno_problems", title).default_height(120.0).cursor(cursor);
        let clicked = panel.show(ui, &[group], |ui, _, index| {
            // Both lists are built from the same source, in the same order.
            if show_fixes
                && let Some(action) = tab.diagnostics.get(index).and_then(|lsp_diag| code_action(text, lsp_diag))
                && ui.add(Button::new(&action.title).small()).clicked()
            {
                fix = Some(action.edit);
            }
        });
        if let Some((_, index)) = clicked
            && let Some(diagnostic) = tab.editor_diagnostics.get(index)
        {
            let pos = LinePos { line: diagnostic.line, column: diagnostic.columns.start };
            egui_code_editor::set_cursor(ui.ctx(), id, pos.to_char_index(&tab.text));
        }
        // `Enter` / `Esc` in the panel: the keyboard is the editor's again.
        if was_focused && !self.problems_cursor.focused {
            ui.memory_mut(|m| m.request_focus(id));
        }

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
    // The file selected in its folder, as VS Code's "Reveal in File Explorer"; the folder if the
    // file isn't there.
    let open_dir = |path: &str| {
        if let Err(err) = gui_shared::fs::reveal_file(std::path::Path::new(path)) {
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
    ui.button(i18n.t(I18nKey::CloseButton)).clicked()
}

impl HkannoPage {
    /// The active tab's file, selected in the file manager (`Shift+Alt+R`, Vim `<Space>V`).
    fn reveal_active(&self, ctx: &mut HkannoPageCtx<'_>) {
        let Some(tab) = self.tabs.get(self.active) else { return };
        if let Err(err) = gui_shared::fs::reveal_file(std::path::Path::new(&tab.settings.input_path)) {
            ctx.notify.error(err);
        }
    }

    /// Was `command` asked for by this frame's keys? It is done then: asked again, `false`.
    fn key_command(&mut self, command: &str) -> bool {
        let Some(at) = self.key_commands.iter().position(|asked| *asked == command) else { return false };
        self.key_commands.swap_remove(at);
        true
    }

    /// Who has the keyboard: what a binding's `when` sees.
    fn key_context(&self, egui_ctx: &egui::Context, ctx: &HkannoPageCtx<'_>) -> keybinding::KeyContext {
        let keymap = to_keymap(ctx.lsp_settings.editor_mode);
        let tab = self.tabs.get(self.active);
        let editor_focus = tab.is_some_and(|tab| egui_ctx.memory(|m| m.has_focus(tab.editor_id())));
        keybinding::KeyContext {
            page: "hkanno",
            editor_focus,
            vim: keymap == Keymap::Vim,
            vim_mode: tab.and_then(|tab| tab.editor.keymap.vim_mode(keymap)),
            problems_focus: self.problems_cursor.focused,
            problems_visible: !self.problems_hidden,
            input_focus: !editor_focus && egui_ctx.egui_wants_keyboard_input(),
            ..Default::default()
        }
    }
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
