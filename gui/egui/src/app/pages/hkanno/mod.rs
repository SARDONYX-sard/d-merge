//! hkanno editor page: edit the annotations embedded in Havok animation files.
//!
//! ```text
//! ┌ [attack.hkx ✖] [idle.hkx ✖]                                     [Open] ┐  tabs
//! ├ Output: [attack.modified.hkx                 ]  Format: [Amd64 ▼]       ┤  file bar
//! ├ [Save] [Revert] [Format] [⚙]                              [Hide Preview] ┤  toolbar
//! │ Annotation                              │ Preview (XML)                    │
//! │ # numOriginalFrames: 38                 │ <hkobject ...>                   │
//! │ trackName: Root                         │   <hkparam name="trackName">...  │
//! │ 0.100000 MCO_DodgeOpen                  │                                  │
//! ├ Problems ─────────────────────────────────────────────────────────────────┤
//! └ ✖ Ln 5, Col 1: Missing z value in animmotion.                  [Fix ...]  ┘
//! ```
//!
//! # Modules
//! - [`tab`]: one open file (load / save / revert / diagnostics)
//! - [`editor`]: the text editor with language features
//! - [`preview`]: XML preview + cursor sync
//! - [`toolbar`]: file bar, toolbar, revert dialog, language feature options

mod editor;
mod preview;
mod tab;
mod toolbar;

use d_merge_gui_shared::{i18n::I18nKey, settings::hkanno::HkannoSettings};
use hkanno_lsp::features::{code_action::code_action, diagnostics::Severity};

use self::{
    editor::{EditorOutput, text_pos::LinePos},
    tab::HkannoTab,
};
use crate::{
    app::App,
    theme::{themed_central_panel, themed_top_bottom_panel},
    ui::{notify::Notification, shadcn::Button},
};

/// Open files and transient dialog state.
#[derive(Default)]
pub(crate) struct HkannoPage {
    tabs: Vec<HkannoTab>,
    active: usize,
    /// "Language features" window.
    show_lsp_options: bool,
    /// "Revert?" confirmation modal.
    show_revert: bool,
}

impl HkannoPage {
    /// Re-opens the tabs saved in `settings.json`. Files that can no longer be read are skipped
    /// (and reported in `notify`).
    pub(crate) fn restore(settings: &HkannoSettings, notify: &mut Notification) -> Self {
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

    /// Settings to persist (tabs + active tab). Options are kept from `current`.
    pub(crate) fn to_settings(&self, current: &HkannoSettings) -> HkannoSettings {
        HkannoSettings {
            tabs: self.tabs.iter().map(|tab| tab.settings.clone()).collect(),
            active_tab: self.active,
            ..current.clone()
        }
    }

    fn active_tab(&mut self) -> Option<&mut HkannoTab> {
        self.tabs.get_mut(self.active)
    }

    /// Opens `paths`; already open files just become active.
    fn open(
        &mut self,
        paths: impl IntoIterator<Item = String>,
        notify: &mut Notification,
        failed: &str,
    ) {
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
                    notify.error(format!("{failed}: {err}"));
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
}

impl App {
    /// Draws the hkanno editor page.
    pub(crate) fn ui_hkanno_page(&mut self, ui: &mut egui::Ui) {
        let theme = self.settings.ui.theme;
        let bg = self.theme_manager.current_bg_color().cloned();
        let panel_top =
            |id: &'static str| themed_top_bottom_panel(egui::Panel::top(id), theme, bg.as_ref());

        self.handle_hkanno_drop(ui.ctx());
        self.handle_hkanno_shortcuts(ui.ctx());

        // ── Fixed bars (registered before the central area) ────────────────────
        panel_top("hkanno_tabs").show(ui, |ui| self.ui_hkanno_tab_bar(ui));
        if !self.hkanno.tabs.is_empty() {
            panel_top("hkanno_file_bar").show(ui, |ui| self.ui_hkanno_file_bar(ui));
            panel_top("hkanno_toolbar").show(ui, |ui| self.ui_hkanno_toolbar(ui));
            self.ui_hkanno_problems(ui);
            if self.settings.hkanno.show_preview {
                self.ui_hkanno_preview(ui);
            }
        }

        // ── Editor ─────────────────────────────────────────────────────────────
        themed_central_panel(egui::CentralPanel::default(), theme, bg.as_ref()).show(ui, |ui| {
            if self.hkanno.tabs.is_empty() {
                ui.centered_and_justified(|ui| ui.weak(self.i18n.t(I18nKey::HkannoEmptyMessage)));
            } else {
                self.ui_hkanno_editor(ui);
            }
        });

        self.ui_hkanno_dialogs(ui.ctx());
        self.ui_hkanno_drop_overlay(ui.ctx());
    }

    /// `[attack.hkx ✖] [idle.hkx* ✖] ...  [Open]`
    fn ui_hkanno_tab_bar(&mut self, ui: &mut egui::Ui) {
        let mut close = None;
        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal().id_salt("hkanno_tab_scroll").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, tab) in self.hkanno.tabs.iter().enumerate() {
                        // `*` marks unsaved edits.
                        let title = if tab.dirty {
                            format!("{}*", tab.title())
                        } else {
                            tab.title().to_string()
                        };
                        if ui
                            .add(Button::selectable(i == self.hkanno.active, title))
                            .on_hover_text(&tab.settings.input_path)
                            .clicked()
                        {
                            self.hkanno.active = i;
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
                    .add(Button::new(self.i18n.t(I18nKey::HkannoOpenButton)))
                    .on_hover_text(self.i18n.t(I18nKey::HkannoOpenHover))
                    .clicked()
                    && let Some(files) = rfd::FileDialog::new()
                        .add_filter("Havok Animation", &["hkx", "xml"])
                        .pick_files()
                {
                    let failed = self.i18n.t(I18nKey::HkannoLoadFailed);
                    let paths = files.into_iter().map(|p| p.display().to_string());
                    self.hkanno.open(paths, &mut self.notify, failed);
                }
            });
        });

        if let Some(index) = close {
            self.hkanno.close(index);
        }
    }

    /// The text editor of the active tab.
    fn ui_hkanno_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new(self.i18n.t(I18nKey::HkannoAnnotationLabel)).weak());

        let options = self.settings.hkanno.lsp.clone();
        let show_preview = self.settings.hkanno.show_preview;
        let Some(tab) = self.hkanno.active_tab() else { return };
        let id = editor_id(&tab.settings.input_path);

        // Restore the saved cursor once, when the tab is first shown.
        if let Some(index) = tab.editor.take_pending_cursor(tab.settings.cursor) {
            editor::set_cursor(ui.ctx(), id, index);
        }

        let EditorOutput { changed, cursor } = egui::ScrollArea::both()
            .id_salt("hkanno_editor_scroll")
            .auto_shrink(false)
            .show(ui, |ui| {
                editor::editor_ui(
                    ui,
                    id,
                    &mut tab.text,
                    &tab.diagnostics,
                    &options,
                    &mut tab.editor,
                )
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
    }

    /// Right pane: XML preview of the active tab.
    fn ui_hkanno_preview(&mut self, ui: &mut egui::Ui) {
        let title = self.i18n.t(I18nKey::HkannoPreviewTitle);
        let error_title = self.i18n.t(I18nKey::HkannoPreviewError);
        let Some(tab) = self.hkanno.tabs.get_mut(self.hkanno.active) else { return };

        // Split borrows: the closure reads the document while `preview` is mutated.
        let tab::HkannoTab { preview, text, original, bytes, settings, .. } = tab;
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
    fn ui_hkanno_problems(&mut self, ui: &mut egui::Ui) {
        let lsp = &self.settings.hkanno.lsp;
        let (show_diagnostics, show_fixes) = (lsp.diagnostics, lsp.code_action);
        let title = self.i18n.t(I18nKey::HkannoProblemsLabel);
        let Some(tab) = self.hkanno.tabs.get_mut(self.hkanno.active) else { return };
        if !show_diagnostics || tab.diagnostics.is_empty() {
            return;
        }

        let id = editor_id(&tab.settings.input_path);
        let mut fix = None;

        egui::Panel::bottom("hkanno_problems").resizable(true).default_size(120.0).show(ui, |ui| {
            ui.weak(format!("{title} ({})", tab.diagnostics.len()));
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                for diagnostic in &tab.diagnostics {
                    ui.horizontal(|ui| {
                        let icon = match diagnostic.severity {
                            Severity::Error => "✖",
                            Severity::Warning => "⚠",
                        };
                        ui.colored_label(
                            editor::severity_color(ui.visuals(), diagnostic.severity),
                            icon,
                        );

                        let pos = LinePos {
                            line: diagnostic.range.line,
                            column: diagnostic.range.span.start,
                        };
                        let location = format!(
                            "Ln {}, Col {}: {}",
                            pos.line + 1,
                            pos.column + 1,
                            diagnostic.message
                        );
                        if ui.link(location).clicked() {
                            editor::set_cursor(ui.ctx(), id, pos.to_char_index(&tab.text));
                        }

                        if show_fixes
                            && let Some(action) = code_action(&tab.text, diagnostic)
                            && ui.add(Button::new(action.title).small()).clicked()
                        {
                            fix = Some(action.edit);
                        }
                    });
                }
            });
        });

        if let Some(edit) = fix {
            let range = edit.range;
            let start =
                LinePos { line: range.line, column: range.span.start }.to_char_index(&tab.text);
            let end = LinePos { line: range.line, column: range.span.end }.to_char_index(&tab.text);
            let (start, end) = (
                editor::text_pos::char_to_byte(&tab.text, start),
                editor::text_pos::char_to_byte(&tab.text, end),
            );
            tab.text.replace_range(start..end, &edit.new_text);
            tab.dirty = true;
            tab.text_changed();
        }
    }

    /// Opens files dropped onto the window (only while this page is shown).
    fn handle_hkanno_drop(&mut self, ctx: &egui::Context) {
        let dropped: Vec<String> = ctx.input(|i| {
            i.raw.dropped_files.iter().map(|f| f.path().display().to_string()).collect()
        });
        if !dropped.is_empty() {
            let failed = self.i18n.t(I18nKey::HkannoLoadFailed);
            self.hkanno.open(dropped, &mut self.notify, failed);
        }
    }

    /// Dashed overlay while files are dragged over the window.
    fn ui_hkanno_drop_overlay(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("hkanno_drop"),
        ));
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
            self.i18n.t(I18nKey::HkannoDropHint),
            egui::FontId::proportional(24.0),
            blue,
        );
    }
}

/// Stable id of a tab's editor, so cursor/undo state follows the file (not the tab position).
fn editor_id(input_path: &str) -> egui::Id {
    egui::Id::unique(("hkanno_editor", input_path))
}
