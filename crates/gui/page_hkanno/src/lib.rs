//! hkanno editor page: edit the annotations embedded in Havok animation files.
//!
//! ```text
//! ┌ [attack.hkx ✖] [idle.hkx ✖]                                       [Open] ┐  tabs
//! ├ Output: [attack.modified.hkx                 ]  Format: [Amd64 ▼]         ┤  file bar
//! ├ [Save] [Revert] [Format] [⚙] Keymap: [Vim ▼]               [Hide Preview] ┤  toolbar
//! │ Annotation                              │ Preview (XML)                      │
//! │ # numOriginalFrames: 38                 │ <hkobject ...>                     │
//! │ trackName: Root                         │   <hkparam name="trackName">...    │
//! │ 0.100000 MCO_DodgeOpen                  │                                    │
//! ├ Problems ───────────────────────────────────────────────────────────────────┤
//! └ ✖ Ln 5, Col 1: Missing z value in animmotion.                    [Fix ...]  ┘
//! ```
//!
//! # Ownership
//! - persisted state: `settings.hkanno`, borrowed through [`HkannoPageCtx`]
//! - open files / editor buffers: [`HkannoPage`], owned by the app
//!
//! # Modules
//! - [`tab`]: one open file (load / save / revert / diagnostics)
//! - [`language`]: plugs `hkanno_lsp` into the generic `egui_code_editor::CodeEditor`
//! - [`preview`]: XML preview + cursor sync
//! - [`toolbar`]: file bar, toolbar, revert dialog, language feature options

mod language;
mod preview;
mod tab;
mod toolbar;

use egui_code_editor::{
    CodeEditor, CodeEditorOutput, EditorCommand, EditorFeatures, Keymap,
    text_pos::{LinePos, char_to_byte},
};
use egui_widgets::{notify::Notification, shadcn::Button};
use gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::hkanno::{EditorMode, HkannoSettings},
};
use hkanno_lsp::features::code_action::code_action;

use self::{language::HkannoLanguage, tab::HkannoTab};

/// What the page borrows from the app for one frame.
#[derive(Debug)]
pub struct HkannoPageCtx<'a> {
    /// Persisted page settings (`settings.hkanno`): options, preview toggle, keymap.
    pub settings: &'a mut HkannoSettings,
    pub i18n: &'a I18nMap,
    /// The app's notification bar.
    pub notify: &'a mut Notification,
    /// Frame (background) of the page's panels, from the app theme.
    pub panel_frame: egui::Frame,
}

/// Open files and transient dialog state. Keep one in the app.
#[derive(Default)]
pub struct HkannoPage {
    tabs: Vec<HkannoTab>,
    active: usize,
    /// "Language features" window.
    show_lsp_options: bool,
    /// "Revert?" confirmation modal.
    show_revert: bool,
}

impl std::fmt::Debug for HkannoPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HkannoPage")
            .field("tabs", &self.tabs.len())
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl HkannoPage {
    /// Re-opens the tabs saved in `settings.json`. Files that can no longer be read are skipped
    /// (and reported in `notify`).
    pub fn restore(settings: &HkannoSettings, notify: &mut Notification) -> Self {
        let mut tabs = Vec::new();
        for saved in &settings.tabs {
            match HkannoTab::restore(saved) {
                Ok(tab) => tabs.push(tab),
                Err(err) => {
                    tracing::error!("Failed to restore hkanno tab: {err}");
                    notify.error(err);
                }
            }
        }
        let active = settings.active_tab.min(tabs.len().saturating_sub(1));
        Self { tabs, active, ..Self::default() }
    }

    /// Writes the open tabs into `settings` (call before saving `settings.json`).
    pub fn store(&self, settings: &mut HkannoSettings) {
        settings.tabs = self.tabs.iter().map(|tab| tab.settings.clone()).collect();
        settings.active_tab = self.active;
    }

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
            egui::Panel::top("hkanno_toolbar")
                .frame(frame)
                .show(ui, |ui| self.toolbar(ui, &mut ctx));
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

    fn active_tab(&mut self) -> Option<&mut HkannoTab> {
        self.tabs.get_mut(self.active)
    }

    /// Opens `paths`; already open files just become active.
    fn open(&mut self, paths: impl IntoIterator<Item = String>, ctx: &mut HkannoPageCtx<'_>) {
        let failed = ctx.i18n.t(I18nKey::HkannoLoadFailed);
        for path in paths {
            if let Some(index) = self.tabs.iter().position(|t| t.settings.input_path == path) {
                self.active = index;
                continue;
            }
            match HkannoTab::open(&path) {
                Ok(tab) => {
                    self.tabs.push(tab);
                    self.active = self.tabs.len() - 1;
                }
                Err(err) => {
                    tracing::error!("{failed}: {err}");
                    ctx.notify.error(format!("{failed}: {err}"));
                }
            }
        }
    }

    fn close(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
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
                        let title = if tab.dirty {
                            format!("{}*", tab.title())
                        } else {
                            tab.title().to_string()
                        };
                        if ui
                            .add(Button::selectable(i == self.active, title))
                            .on_hover_text(&tab.settings.input_path)
                            .clicked()
                        {
                            self.active = i;
                        }
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
                    open = rfd::FileDialog::new()
                        .add_filter("Havok Animation", &["hkx", "xml"])
                        .pick_files();
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

        let lsp = &ctx.settings.lsp;
        let language =
            HkannoLanguage { semantic: lsp.semantic_tokens, code_action: lsp.code_action };
        let features = EditorFeatures {
            completion: lsp.completion,
            hover: lsp.hover,
            diagnostics: lsp.diagnostics,
            format: lsp.formatter,
            inlay_hints: lsp.inlay_hints,
            signature_help: lsp.signature_help,
        };
        let keymap = to_keymap(ctx.settings.editor_mode);
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

        let CodeEditorOutput { changed, cursor, command } = egui::ScrollArea::both()
            .id_salt("hkanno_editor_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                CodeEditor::new(tab.settings.input_path.as_str(), &mut tab.text)
                    .language(&language)
                    .diagnostics(&tab.editor_diagnostics)
                    .features(features)
                    .keymap(keymap)
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
        preview.poll(
            ui.ctx(),
            || tab::document(original, text).into_static(),
            bytes,
            &settings.input_path,
            text,
        );

        egui::Panel::right("hkanno_preview").resizable(true).default_size(520.0).show(ui, |ui| {
            match &preview.error {
                Some(err) => {
                    ui.colored_label(ui.visuals().error_fg_color, format!("{error_title}: {err}"))
                }
                None => ui.weak(title),
            };
            preview.ui(ui);
        });
    }

    /// Bottom pane: diagnostics of the active tab with quick fixes.
    fn problems(&mut self, ui: &mut egui::Ui, ctx: &HkannoPageCtx<'_>) {
        let (show_diagnostics, show_fixes) =
            (ctx.settings.lsp.diagnostics, ctx.settings.lsp.code_action);
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
                        let pos =
                            LinePos { line: diagnostic.line, column: diagnostic.columns.start };
                        let location = format!(
                            "Ln {}, Col {}: {}",
                            pos.line + 1,
                            pos.column + 1,
                            diagnostic.message
                        );
                        if ui.link(location).clicked() {
                            egui_code_editor::set_cursor(
                                ui.ctx(),
                                id,
                                pos.to_char_index(&tab.text),
                            );
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
            let at = |column| LinePos { line: edit.range.line, column }.to_char_index(&tab.text);
            let start = char_to_byte(&tab.text, at(edit.range.span.start));
            let end = char_to_byte(&tab.text, at(edit.range.span.end));
            tab.text.replace_range(start..end, &edit.new_text);
            tab.dirty = true;
            tab.text_changed();
        }
    }

    /// Opens files dropped onto the window (only while this page is shown).
    fn handle_drop(&mut self, egui_ctx: &egui::Context, ctx: &mut HkannoPageCtx<'_>) {
        let dropped: Vec<String> = egui_ctx.input(|i| {
            i.raw.dropped_files.iter().map(|f| f.path().display().to_string()).collect()
        });
        if !dropped.is_empty() {
            self.open(dropped, ctx);
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
    painter.rect_stroke(
        rect.shrink(4.0),
        8.0,
        egui::Stroke::new(3.0_f32, blue),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        i18n.t(I18nKey::HkannoDropHint),
        egui::FontId::proportional(24.0),
        blue,
    );
}
