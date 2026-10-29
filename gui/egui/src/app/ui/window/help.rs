//! help dialog, confirmation dialog.

use std::path::{Path, PathBuf};

use d_merge_gui_shared::{
    fs::open_existing_dir_or_ancestor,
    i18n::{I18nKey, I18nMap},
    settings::{
        support_pandora::{Pandora, PandoraVersion},
        ui::FontMode,
    },
};
use egui::Color32;

use crate::{
    app::App,
    ui::shadcn::{Button, Checkbox, ComboBox, TextEdit},
};

const GRID_SPACING: [f32; 2] = [8.0, 6.0];

impl App {
    /// Renders the help / about window anchored to the viewport centre.
    pub(crate) fn ui_help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        if help_overlay(ctx).clicked() {
            self.show_help = false;
            return;
        }

        // NOTE: `Window::open` holds `&mut bool` for the whole `show` call, which would conflict with
        // the closure capturing `&mut self`. `bool` is `Copy`, so go through a local.
        let mut open = true;
        let size =
            egui::vec2(self.settings.ui.window.width * 0.5, self.settings.ui.window.height * 0.85);

        egui::Window::new(self.i18n.t(I18nKey::HelpButton))
            .open(&mut open)
            .collapsible(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().id_salt("help_window_scroll").show(ui, |ui| {
                    fn section_title(ui: &mut egui::Ui, title: &str) {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(title).strong().size(20.0));
                        ui.add_space(4.0);
                    }

                    fn section_separator(ui: &mut egui::Ui) {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                    }

                    section_title(ui, "About");
                    self.ui_help_info(ui);
                    section_separator(ui);

                    section_title(ui, self.i18n.t(I18nKey::BugReportTitleLabel));
                    self.ui_bug_report(ui);
                    section_separator(ui);

                    section_title(ui, self.i18n.t(I18nKey::FontTitleLabel));
                    self.ui_font_section(ui);
                    section_separator(ui);

                    section_title(ui, self.i18n.t(I18nKey::LoggingTitleLabel));
                    self.ui_log_section(ui);
                    section_separator(ui);

                    section_title(ui, self.i18n.t(I18nKey::I18nTitleLabel));
                    self.ui_translation_section(ui);
                    section_separator(ui);

                    section_title(ui, self.i18n.t(I18nKey::BackgroundTitleLabel));
                    self.ui_background_section(ui);
                    section_separator(ui);

                    section_title(ui, "Pandora");
                    self.ui_pandora_section(ui);
                    section_separator(ui);
                });
            });

        self.show_help &= open;
    }

    fn ui_help_info(&self, ui: &mut egui::Ui) {
        let rows = [
            ("D Merge Version:", env!("CARGO_PKG_VERSION"), None),
            (
                self.i18n.t(I18nKey::ChangeLogLabel),
                "CHANGELOG.md",
                Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/CHANGELOG.md")),
            ),
            (
                self.i18n.t(I18nKey::ModTestStatusLabel),
                "test_status.md",
                Some(concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/blob/",
                    env!("CARGO_PKG_VERSION"),
                    "/docs/test_status.md"
                )),
            ),
            (
                self.i18n.t(I18nKey::SourceCodeLabel),
                "GitHub",
                Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/tree/", env!("CARGO_PKG_VERSION"))),
            ),
            (
                self.i18n.t(I18nKey::LicenseLabel),
                env!("CARGO_PKG_LICENSE"),
                Some(concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/blob/",
                    env!("CARGO_PKG_VERSION"),
                    "/LICENSE"
                )),
            ),
            (self.i18n.t(I18nKey::AuthorLabel), env!("CARGO_PKG_AUTHORS"), None),
        ];

        egui::Grid::new("help_info_grid").num_columns(2).spacing(GRID_SPACING).show(ui, |ui| {
            for (label, value, url) in rows {
                ui.label(label);

                match url {
                    Some(url) => ui.hyperlink_to(value, url).on_hover_text(url),
                    None => ui.label(value),
                };
                ui.end_row();
            }
        });
    }

    fn ui_bug_report(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(Button::new(self.i18n.t(I18nKey::IssueReportButton)))
                .on_hover_text(self.i18n.t(I18nKey::IssueReportHover))
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl {
                    url: self.settings.create_issue_link(),
                    new_tab: true,
                });
            }

            const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");
            ui.hyperlink_to(self.i18n.t(I18nKey::BugReportSeeIssues), ISSUE_URL)
                .on_hover_text(ISSUE_URL);
        });
    }

    fn ui_font_section(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("font_grid").num_columns(2).spacing(GRID_SPACING).show(ui, |ui| {
            ui.label(self.i18n.t(I18nKey::FontModeLabel))
                .on_hover_text(self.i18n.t(I18nKey::FontModeHover));
            let modes = [
                (FontMode::Default, self.i18n.t(I18nKey::FontModeDefault)),
                (FontMode::System, self.i18n.t(I18nKey::FontModeSystem)),
                (FontMode::File, self.i18n.t(I18nKey::FontModeFile)),
            ];
            let font_mode = &mut self.settings.ui.font.mode; // disjoint field from `self.i18n`
            let mut index = modes.iter().position(|(m, _)| m == font_mode).unwrap_or(0);
            let mode_changed = ComboBox::from_id_salt("font_mode")
                .show_index(ui, &mut index, modes.len(), |i| modes[i].1)
                .changed();
            if mode_changed {
                *font_mode = modes[index].0;
            }
            ui.end_row();

            match self.settings.ui.font.mode {
                FontMode::Default => {
                    if mode_changed {
                        self.apply_font(ui.ctx());
                    }
                }
                FontMode::System => {
                    ui.label(self.i18n.t(I18nKey::FontFamily));
                    let fonts = crate::fonts::font_families();
                    let name = &mut self.settings.ui.font.name;
                    let mut index = fonts.iter().position(|f| f == name).unwrap_or(usize::MAX);
                    let name_changed = ComboBox::from_id_salt("font_family")
                        .width(ui.available_width())
                        .search_hint(format!("{}...", self.i18n.t(I18nKey::SearchLabel)))
                        .show_index(ui, &mut index, fonts.len(), |i| {
                            fonts.get(i).map_or("", String::as_str)
                        })
                        .changed();
                    if name_changed {
                        name.clone_from(&fonts[index]);
                    }
                    ui.end_row();

                    if mode_changed || name_changed {
                        self.apply_font(ui.ctx());
                    }
                }
                FontMode::File => {
                    // NOTE: File mode is applied only on reload/pick, never per keystroke.
                    if path_row(ui, &self.i18n, &mut self.settings.ui.font.path, &FONT_ROW) {
                        self.apply_font(ui.ctx());
                    }
                }
            }
        });
    }

    fn ui_log_section(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("log_grid").num_columns(2).spacing(GRID_SPACING).show(ui, |ui| {
            if path_row(ui, &self.i18n, &mut self.settings.log.dir_path, &LOG_ROW) {
                self.reload_log();
            }
        });
    }

    fn ui_translation_section(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("i18n_grid").num_columns(2).spacing(GRID_SPACING).show(ui, |ui| {
            if path_row(ui, &self.i18n, &mut self.settings.ui.i18n_path, &I18N_ROW) {
                self.reload_i18n();
            }

            ui.label("English:");
            ui.horizontal(|ui| {
                if ui
                    .add(Button::new(self.i18n.t(I18nKey::I18nWriteNewJsonButton)))
                    .on_hover_text(self.i18n.t(I18nKey::I18nWriteNewJsonHover))
                    .clicked()
                {
                    self.write_new_i18n();
                }

                if ui
                    .add(Button::new("Force English"))
                    .on_hover_text("Temporarily switch the UI to English, primarily for debugging.")
                    .clicked()
                {
                    self.i18n = I18nMap::new();
                }
            });
            ui.end_row();
        });
    }

    fn ui_background_section(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("background_grid").num_columns(2).spacing(GRID_SPACING).show(ui, |ui| {
            if path_row(
                ui,
                &self.i18n,
                &mut self.settings.ui.background_image.path,
                &BACKGROUND_ROW,
            ) {
                self.settings.ui.background_image.enabled = true;
                self.reload_background(ui.ctx());
            }

            ui.add(Checkbox::new(
                &mut self.settings.ui.background_image.enabled,
                self.i18n.t(I18nKey::BackgroundImageEnabled),
            ))
            .on_hover_text(self.i18n.t(I18nKey::BackgroundImageEnabledHover));
            ui.end_row();
        });
    }

    /// ```txt
    /// Pandora
    /// Target version:      [ 4.4.0Beta ▼ ]
    /// Import path: [ path    ] [Import]
    /// Export path: [ path    ] [Export]
    /// ```
    fn ui_pandora_section(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("pandora_grid").num_columns(2).spacing(GRID_SPACING).show(ui, |ui| {
            ui.label(self.i18n.t(I18nKey::PandoraTargetVersionLabel))
                .on_hover_text(self.i18n.t(I18nKey::PandoraTargetVersionHover));
            let versions = [PandoraVersion::V4_4_0Beta];
            let version = &mut self.settings.pandora.version;
            let mut index = versions.iter().position(|v| v == version).unwrap_or(0);
            if ComboBox::from_id_salt("pandora_version")
                .show_index(ui, &mut index, versions.len(), |i| versions[i].to_static_str())
                .changed()
            {
                *version = versions[index];
            }
            ui.end_row();

            if folder_action_row(
                ui,
                &self.i18n,
                &mut self.settings.pandora.import_path,
                [
                    I18nKey::PandoraImportPathLabel,
                    I18nKey::PandoraImportLabel,
                    I18nKey::PandoraImportHover,
                ],
            ) {
                self.import_pandora();
            }

            // Fixed: the old code opened `import_path` from the *export* label button.
            if folder_action_row(
                ui,
                &self.i18n,
                &mut self.settings.pandora.export_path,
                [
                    I18nKey::PandoraExportPathLabel,
                    I18nKey::PandoraExportLabel,
                    I18nKey::PandoraExportHover,
                ],
            ) {
                self.export_pandora();
            }
        });
    }

    fn apply_font(&mut self, ctx: &egui::Context) {
        let Err(err) = crate::fonts::set_fonts(ctx, &self.settings.ui.font) else {
            return;
        };
        match err {
            crate::fonts::FontError::Warn(msg) => {
                tracing::warn!(msg);
                self.notify.warn(msg);
            }
            crate::fonts::FontError::Error(msg) => {
                tracing::error!(msg);
                self.notify.error(msg);
            }
        }
    }

    fn reload_log(&mut self) {
        let dir = Path::new(self.settings.log.dir_path.as_str());
        if let Err(err) =
            tracing_rotation::global::change_log_path(dir, d_merge_gui_shared::log::LOG_FILENAME)
        {
            tracing::error!(%err);
            self.notify.error(format!("Failed to reload log: {err}"));
        } else {
            self.update_log_dir();
            tracing::info!("Log file rotated.");
            self.notify.success("Log file rotated.".to_string());
        }
    }

    /// Reloads the i18n map from disk and updates [`App::i18n`].
    fn reload_i18n(&mut self) {
        match I18nMap::load(self.settings.ui.i18n_path.as_str()) {
            Ok(i18n) => {
                self.i18n = i18n;
                self.notify.success(format!("Reloaded {}", self.settings.ui.i18n_path));
            }
            Err(err) => {
                self.notify.error(format!("Failed to reload: {err}"));
            }
        }
    }

    fn write_new_i18n(&mut self) {
        let dir = start_dir(&self.settings.ui.i18n_path, ".", false);

        let path = rfd::FileDialog::new()
            .set_directory(dir)
            .set_title("Save translation.json")
            .set_file_name("translation.json")
            .add_filter("translation", &["json"])
            .save_file();

        if let Some(path) = path {
            match I18nMap::save(&path) {
                Ok(()) => {
                    self.notify.success(format!("OK. Wrote {}", path.display()));
                }
                Err(err) => self.notify.error(err.to_string()),
            }
        }
    }

    fn import_pandora(&mut self) {
        let path = self.settings.pandora.import_path.clone();
        let version = self.settings.pandora.version;
        match Pandora::import_to(&path, &mut self.settings, version) {
            Ok(()) => {
                self.notify.success(format!("Imported Pandora settings from {path}"));
            }
            Err(err) => self.notify.error(err),
        }
    }

    fn export_pandora(&mut self) {
        let path = self.settings.pandora.export_path.clone();
        let version = self.settings.pandora.version;
        match Pandora::export_from(&path, &self.settings, version) {
            Ok(()) => {
                self.notify.success(format!("Exported Pandora settings to {path}"));
            }
            Err(err) => self.notify.error(err),
        }
    }
}

enum Picker {
    File { filter: &'static str, exts: &'static [&'static str], fallback_dir: &'static str },
    Folder,
}

/// Declarative description of a path row. All `const`, so no allocation at all.
struct PathRow {
    label: I18nKey,
    reload_hover: I18nKey,
    picker: Picker,
}

const FONT_ROW: PathRow = PathRow {
    label: I18nKey::FontFileLabel,
    reload_hover: I18nKey::FontReloadHover,
    picker: Picker::File {
        filter: "font",
        exts: &["ttc", "ttf", "tto"],
        fallback_dir: "C:/Windows/Fonts",
    },
};
const LOG_ROW: PathRow = PathRow {
    label: I18nKey::LogDirPathLabel,
    reload_hover: I18nKey::LogReloadHover,
    picker: Picker::Folder,
};
const I18N_ROW: PathRow = PathRow {
    label: I18nKey::I18nPathLabel,
    reload_hover: I18nKey::I18nReloadJsonHover,
    picker: Picker::File { filter: "translation", exts: &["json"], fallback_dir: "." },
};
const BACKGROUND_ROW: PathRow = PathRow {
    label: I18nKey::BackgroundImageLabel,
    reload_hover: I18nKey::BackgroundImageReloadHover,
    picker: Picker::File { filter: "image", exts: &["png", "jpg", "jpeg"], fallback_dir: "." },
};

/// `[label][ text edit ][Select][Reload][Clear]`
///
/// Returns `true` if the target should be reloaded (reload clicked or a new path picked).
/// Like `egui::Response::clicked`, the caller consumes it immediately at the call site.
fn path_row(ui: &mut egui::Ui, i18n: &I18nMap, value: &mut String, row: &PathRow) -> bool {
    fn pick(picker: &Picker, current: &str) -> Option<String> {
        let picked = match picker {
            Picker::File { filter, exts, fallback_dir } => rfd::FileDialog::new()
                .set_directory(start_dir(current, fallback_dir, false))
                .add_filter(*filter, exts)
                .pick_file(),
            Picker::Folder => {
                rfd::FileDialog::new().set_directory(start_dir(current, ".", true)).pick_folder()
            }
        };
        picked.map(|p| p.display().to_string())
    }

    if ui
        .add(Button::new(i18n.t(row.label)))
        .on_hover_text(i18n.t(I18nKey::OpenSelectedPathHover))
        .clicked()
        && let Err(err) = open_existing_dir_or_ancestor(Path::new(value.as_str()))
    {
        tracing::error!(err);
    }

    let mut reload = false;
    // Right-to-left: the buttons take their real width first, the text edit gets the rest.
    // So the visual order is `[text edit][Select][Reload][Clear]`.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        if ui.add(Button::new(i18n.t(I18nKey::ClearButton))).clicked() {
            value.clear();
        }

        if ui
            .add(Button::new(i18n.t(I18nKey::ReloadButton)))
            .on_hover_text(i18n.t(row.reload_hover))
            .clicked()
        {
            reload = true;
        }

        if ui.add(Button::new(i18n.t(I18nKey::SelectButton))).clicked()
            && let Some(picked) = pick(&row.picker, value)
        {
            *value = picked;
            reload = true;
        }

        let width = ui.available_width();
        ui.add(TextEdit::singleline(value).desired_width(width));
    });
    ui.end_row();
    reload
}

/// `[label][ text edit ][Action]`. Returns `true` when a folder was picked.
fn folder_action_row(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    path: &mut String,
    [label, action, action_hover]: [I18nKey; 3],
) -> bool {
    if ui
        .add(Button::new(i18n.t(label)))
        .on_hover_text(i18n.t(I18nKey::OpenSelectedPathHover))
        .clicked()
        && let Err(err) = open_existing_dir_or_ancestor(Path::new(path.as_str()))
    {
        tracing::error!(err);
    }

    let mut picked = false;

    // Right-to-left: see `path_row`.
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        if ui.add(Button::new(i18n.t(action))).on_hover_text(i18n.t(action_hover)).clicked()
            && let Some(dir) = rfd::FileDialog::new().set_directory(path.as_str()).pick_folder()
        {
            *path = dir.display().to_string();
            picked = true;
        }

        let width = ui.available_width();
        ui.add(TextEdit::singleline(path).desired_width(width));
    });

    ui.end_row();
    picked
}

/// IMPORTANT: Without canonicalizing, rfd cannot set the directory correctly.
fn start_dir(current: &str, fallback: &str, current_is_dir: bool) -> PathBuf {
    let path = Path::new(current);
    let base = if current_is_dir { Some(path) } else { path.parent() };
    base.map_or_else(
        || PathBuf::from(fallback),
        |p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()),
    )
}

/// Draws the dimmed backdrop; the returned response tells whether it was clicked.
fn help_overlay(ctx: &egui::Context) -> egui::Response {
    egui::Area::new(egui::Id::unique("help_overlay_bg"))
        .order(egui::Order::Background)
        .fixed_pos(ctx.content_rect().min)
        .show(ctx, |ui| {
            let rect = ui.ctx().content_rect();
            ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(160));
            ui.allocate_response(rect.size(), egui::Sense::click())
        })
        .inner
}
