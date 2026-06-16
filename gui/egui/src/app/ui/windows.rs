//! Floating windows: log viewer, help dialog, confirmation dialog.
//!
//! All three are *deferred* or *modal* overlays rendered after the main
//! panels.  They read their open-state from `App` fields and write back
//! through local variables to avoid conflicting borrows inside closures.
//!
//! # Borrow-split pattern
//! egui closures hold `&mut egui::Ui` and call into `self` methods, which
//! would create a second mutable borrow of `self`.  The workaround used
//! throughout this file is:
//! 1. Extract all strings and values needed inside the closure *before* it.
//! 2. Capture mutations in local `bool` / `String` variables.
//! 3. Apply mutations *after* the closure returns.

use std::{borrow::Cow, path::Path};

use d_merge_gui_shared::{
    i18n::{I18nKey, I18nMap},
    settings::ui::FontMode,
};
use egui::Color32;

use crate::app::App;

impl App {
    /// Renders the deferred log-viewer viewport.
    ///
    /// Opens as a separate OS window (egui deferred viewport) so it can be
    /// moved independently.  Visibility is toggled via the atomic bool
    /// [`App::show_log_window`]; closing the window sets it to `false`.
    pub(crate) fn ui_log_window(&self, ctx: &egui::Context) {
        use std::sync::atomic::Ordering;

        if !self.show_log_window.load(Ordering::Relaxed) {
            return;
        }

        let show_log_window = std::sync::Arc::clone(&self.show_log_window);
        let log_lines = std::sync::Arc::clone(&self.log_lines);
        let clear_button_name = self.i18n.t(I18nKey::ClearButton).to_string();

        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("log_viewer"),
            egui::ViewportBuilder {
                title: Some("Log viewer".to_string()),
                inner_size: Some(egui::Vec2::new(1300.0, 800.0)),
                resizable: Some(true),
                ..Default::default()
            },
            move |ctx, class| {
                assert!(
                    class == egui::ViewportClass::Deferred,
                    "This egui backend doesn't support multiple viewports"
                );

                egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(clear_button_name.as_str()).clicked() {
                            log_lines.write().clear();
                        }
                        if ui.button("Copy").clicked() {
                            let text = log_lines.read().join("\n");
                            ui.ctx().copy_text(text);
                        }
                    });

                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        let text = log_lines.read().join("\n");
                        ui.label(text);
                    });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    show_log_window.store(false, Ordering::Relaxed);
                }
            },
        );
    }

    /// Renders the help / about window.
    ///
    /// Anchored to the viewport centre.
    pub(crate) fn ui_help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }

        // --- overlay (background) ---
        if egui::Area::new("help_overlay_bg".into())
            .order(egui::Order::Background)
            .fixed_pos(ctx.content_rect().min)
            .show(ctx, |ui| {
                let rect = ui.ctx().content_rect();

                let bg = egui::Color32::from_black_alpha(160);
                ui.painter().rect_filled(rect, 0.0, bg);
                ui.allocate_response(rect.size(), egui::Sense::click())
            })
            .inner
            .clicked()
        {
            self.show_help = false;
            return;
        }

        let mut show_help = true;

        // This clone, which may seem unnecessary at first glance, is a measure designed to allow the use of `&self` within the closure.
        let mut selected_font_mode = self.settings.ui.font.mode;
        let mut selected_font_name = self.settings.ui.font.name.clone();
        let mut selected_font_path = self.settings.ui.font.path.clone();
        let mut reload_font_clicked = false;

        let mut selected_log_dir_path = self.settings.log.dir_path.clone();
        let mut reload_log_clicked = false;

        let mut selected_i18n_path = self.settings.ui.i18n_path.clone();
        let mut write_i18n_clicked = false;
        let mut reload_i18n_clicked = false;

        egui::Window::new(self.i18n.t(I18nKey::HelpButton))
            .open(&mut show_help)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(crate::APP_TITLE);
                });

                ui.add_space(8.0);
                ui.separator();

                self.ui_help_info(ui);

                ui.add_space(8.0);
                ui.separator();

                self.ui_bug_report(ui);

                ui.add_space(8.0);
                ui.separator();

                self.ui_tooling(UiToolingArgs {
                    ui,
                    selected_font_mode: &mut selected_font_mode,
                    selected_font_name: &mut selected_font_name,
                    selected_font_path: &mut selected_font_path,
                    reload_font_clicked: &mut reload_font_clicked,
                    //
                    selected_log_dir_path: &mut selected_log_dir_path,
                    reload_log_clicked: &mut reload_log_clicked,
                    //
                    selected_i18n_path: &mut selected_i18n_path,
                    reload_i18n_clicked: &mut reload_i18n_clicked,
                    write_i18n_clicked: &mut write_i18n_clicked,
                });
            });

        if !show_help {
            self.show_help = false;
        }

        // Fonts actions
        let did_font_mode_change = self.settings.ui.font.mode != selected_font_mode;
        let did_font_name_change = self.settings.ui.font.name != selected_font_name;

        if did_font_mode_change {
            self.settings.ui.font.mode = selected_font_mode;
        }
        if did_font_name_change {
            self.settings.ui.font.name = selected_font_name;
        }
        if self.settings.ui.font.path != selected_font_path {
            self.settings.ui.font.path = selected_font_path;
        }
        // NOTE: The font path won't be applied automatically until we click the reload button. (Because it would make it harder to use.)
        if ((did_font_mode_change
            && matches!(selected_font_mode, FontMode::Default | FontMode::System))
            || did_font_name_change
            || reload_font_clicked)
            && let Err(err) = crate::fonts::setup_fonts(ctx, &self.settings.ui.font)
        {
            match err {
                crate::fonts::FontError::Warn(msg) => {
                    tracing::warn!(msg);
                    self.notify = (msg, egui::Color32::YELLOW);
                }
                crate::fonts::FontError::Error(msg) => {
                    tracing::error!(msg);
                    self.notify_error(msg);
                }
            }
        }

        // Log actions
        if self.settings.log.dir_path != selected_log_dir_path {
            self.settings.log.dir_path = selected_log_dir_path;
        }
        if reload_log_clicked {
            self.reload_log();
        }

        // i18n actions
        if self.settings.ui.i18n_path != selected_i18n_path {
            self.settings.ui.i18n_path = selected_i18n_path;
        }
        if reload_i18n_clicked {
            self.reload_i18n();
        }
        if write_i18n_clicked {
            let path = Path::new(&self.settings.ui.i18n_path).parent().map_or_else(
                || Cow::Borrowed(Path::new(".")),
                |p| p.canonicalize().map_or(Cow::Borrowed(p), Cow::Owned),
            );

            let path = rfd::FileDialog::new()
                .set_directory(path)
                .set_title("Save translation.json")
                .set_file_name("translation.json")
                .add_filter("JSON", &["json"])
                .save_file();

            if let Some(path) = path {
                match I18nMap::save(&path) {
                    Ok(()) => self.set_colored_notify(
                        format!("OK. Wrote {}", path.display()),
                        Color32::GREEN,
                    ),
                    Err(err) => self.notify_error(err.to_string()),
                }
            }
        }
    }

    fn ui_help_info(&self, ui: &mut egui::Ui) {
        let rows = [
            (self.i18n.t(I18nKey::AuthorLabel).to_string(), env!("CARGO_PKG_AUTHORS"), None),
            (
                self.i18n.t(I18nKey::LicenseLabel).to_string(),
                env!("CARGO_PKG_LICENSE"),
                Some(concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/blob/",
                    env!("CARGO_PKG_VERSION"),
                    "/LICENSE"
                )),
            ),
            (
                self.i18n.t(I18nKey::SourceCodeLabel).to_string(),
                "GitHub",
                Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/tree/", env!("CARGO_PKG_VERSION"))),
            ),
            (
                self.i18n.t(I18nKey::ChangeLogLabel).to_string(),
                "CHANGELOG.md",
                Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/CHANGELOG.md")),
            ),
            (
                self.i18n.t(I18nKey::ModTestStatusLabel).to_string(),
                "test_status.md",
                Some(concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/blob/",
                    env!("CARGO_PKG_VERSION"),
                    "/docs/test_status.md"
                )),
            ),
        ];

        egui::Grid::new("help_info_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            for (label, value, url) in rows {
                ui.label(label);

                match url {
                    Some(url) => {
                        ui.hyperlink_to(value, url).on_hover_text(url);
                    }
                    None => {
                        ui.label(value);
                    }
                }

                ui.end_row();
            }
        });
    }

    fn ui_bug_report(&self, ui: &mut egui::Ui) {
        let bug_report_label = self.i18n.t(I18nKey::BugReportLabel);

        let see_issues_label = self.i18n.t(I18nKey::BugReportSeeIssues);

        let issue_report_label = self.i18n.t(I18nKey::IssueReportButton);
        let issue_report_hover = self.i18n.t(I18nKey::IssueReportHover);

        ui.label(bug_report_label);

        ui.horizontal(|ui| {
            const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");
            ui.hyperlink_to(see_issues_label, ISSUE_URL).on_hover_text(ISSUE_URL);

            if ui.button(issue_report_label).on_hover_text(issue_report_hover).clicked() {
                ui.ctx().open_url(egui::OpenUrl {
                    url: self.settings.create_issue_link(),
                    new_tab: true,
                });
            }
        });
    }

    fn ui_tooling(&self, args: UiToolingArgs) {
        let UiToolingArgs {
            ui,
            selected_font_mode,
            selected_font_name,
            selected_font_path,
            reload_font_clicked,
            selected_log_dir_path,
            reload_log_clicked,
            selected_i18n_path,
            reload_i18n_clicked,
            write_i18n_clicked,
        } = args;

        ui.label(self.i18n.t(I18nKey::ToolingLabel));
        ui.add_space(4.0);

        egui::Grid::new("tooling_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            self.font_ui(
                ui,
                selected_font_mode,
                selected_font_name,
                selected_font_path,
                reload_font_clicked,
            );

            if path_selector_row(
                PathSelector {
                    ui,
                    label: self.i18n.t(I18nKey::LogDirPathLabel),
                    value: selected_log_dir_path,
                    select_label: self.i18n.t(I18nKey::SelectButton),
                    reload_label: self.i18n.t(I18nKey::ReloadButton),
                    reload_hover: self.i18n.t(I18nKey::LogReloadHover),
                    clear_label: self.i18n.t(I18nKey::ClearButton),
                },
                || {
                    let dir = Path::new(self.settings.log.dir_path.as_str());
                    // IMPORTANT: Without this, rfd cannot set dir correctly.
                    let dir = dir.canonicalize().map(Cow::Owned).unwrap_or(Cow::Borrowed(dir));

                    rfd::FileDialog::new()
                        .set_directory(dir)
                        .pick_folder()
                        .map(|p| p.display().to_string())
                },
            ) {
                *reload_log_clicked = true;
            }

            if path_selector_row(
                PathSelector {
                    ui,
                    label: self.i18n.t(I18nKey::I18nPathLabel),
                    value: selected_i18n_path,
                    select_label: self.i18n.t(I18nKey::SelectButton),
                    reload_label: self.i18n.t(I18nKey::ReloadButton),
                    reload_hover: self.i18n.t(I18nKey::I18nReloadJsonHover),
                    clear_label: self.i18n.t(I18nKey::ClearButton),
                },
                || {
                    let dir = Path::new(&self.settings.ui.i18n_path);
                    // IMPORTANT: Without this, rfd cannot set dir correctly.
                    let dir = dir.canonicalize().map(Cow::Owned).unwrap_or(Cow::Borrowed(dir));

                    rfd::FileDialog::new()
                        .set_directory(dir)
                        .add_filter("translation.json", &["json"])
                        .pick_file()
                        .map(|p| p.display().to_string())
                },
            ) {
                *reload_i18n_clicked = true;
            }

            ui.horizontal(|ui| {
                if ui
                    .button(self.i18n.t(I18nKey::I18nWriteNewJsonButton))
                    .on_hover_text(self.i18n.t(I18nKey::I18nWriteNewJsonHover))
                    .clicked()
                {
                    *write_i18n_clicked = true;
                }
            });
            ui.end_row();
        });
    }

    fn font_ui(
        &self,
        ui: &mut egui::Ui,
        selected_font_mode: &mut FontMode,
        selected_font_name: &mut String,
        selected_font_path: &mut String,
        reload_font_clicked: &mut bool,
    ) {
        ui.label(self.i18n.t(I18nKey::FontModeLabel))
            .on_hover_text(self.i18n.t(I18nKey::FontModeHover));

        egui::ComboBox::from_id_salt("font_mode")
            .selected_text(match selected_font_mode {
                FontMode::Default => self.i18n.t(I18nKey::FontModeDefault),
                FontMode::System => self.i18n.t(I18nKey::FontModeSystem),
                FontMode::File => self.i18n.t(I18nKey::FontModeFile),
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    selected_font_mode,
                    FontMode::Default,
                    self.i18n.t(I18nKey::FontModeDefault),
                );

                ui.selectable_value(
                    selected_font_mode,
                    FontMode::System,
                    self.i18n.t(I18nKey::FontModeSystem),
                );

                ui.selectable_value(
                    selected_font_mode,
                    FontMode::File,
                    self.i18n.t(I18nKey::FontModeFile),
                );
            });

        ui.end_row();

        match selected_font_mode {
            FontMode::Default => {}

            FontMode::System => {
                ui.label(self.i18n.t(I18nKey::FontFamily));

                egui::ComboBox::from_id_salt("font_family")
                    .selected_text(if selected_font_name.is_empty() {
                        self.i18n.t(I18nKey::SelectButton)
                    } else {
                        selected_font_name.as_str()
                    })
                    .show_ui(ui, |ui| {
                        for family in crate::fonts::font_families() {
                            ui.selectable_value(selected_font_name, family.clone(), family);
                        }
                    });

                ui.end_row();
            }

            FontMode::File => {
                if path_selector_row(
                    PathSelector {
                        ui,
                        label: self.i18n.t(I18nKey::FontFileLabel),
                        value: selected_font_path,
                        select_label: self.i18n.t(I18nKey::SelectButton),
                        reload_label: self.i18n.t(I18nKey::ReloadButton),
                        reload_hover: self.i18n.t(I18nKey::FontReloadHover),
                        clear_label: self.i18n.t(I18nKey::ClearButton),
                    },
                    || {
                        // IMPORTANT: Without this, rfd cannot set dir correctly.
                        let dir = Path::new(&self.settings.ui.font.path).parent().map_or_else(
                            || Cow::Borrowed(Path::new("C:/Windows/Fonts")),
                            |p| p.canonicalize().map_or(Cow::Borrowed(p), Cow::Owned),
                        );

                        rfd::FileDialog::new()
                            .set_directory(dir)
                            .add_filter("font.ttc", &["ttc", "ttf", "tto"])
                            .pick_file()
                            .map(|p| p.display().to_string())
                    },
                ) {
                    *reload_font_clicked |= true;
                }
            }
        }
    }

    fn reload_log(&mut self) {
        let dir = Path::new(self.settings.log.dir_path.as_str());
        if let Err(err) =
            tracing_rotation::global::change_log_path(dir, d_merge_gui_shared::log::LOG_FILENAME)
        {
            tracing::error!(%err);
            self.notify_error(format!("Failed to reload log: {err}"));
        } else {
            self.update_log_dir();
            tracing::info!("Log file rotated.");
            self.set_colored_notify("Log file rotated.".to_string(), Color32::GREEN);
        }
    }

    /// Reloads the i18n map from disk and updates [`App::i18n`].
    fn reload_i18n(&mut self) {
        match I18nMap::load(self.settings.ui.i18n_path.as_str()) {
            Ok(i18n) => {
                self.i18n = i18n;
                self.set_colored_notify(
                    format!("Reloaded {}", self.settings.ui.i18n_path),
                    Color32::GREEN,
                );
            }
            Err(err) => {
                self.notify_error(format!("Failed to reload: {err}"));
            }
        }
    }
}

struct UiToolingArgs<'a> {
    ui: &'a mut egui::Ui,
    selected_font_mode: &'a mut FontMode,
    selected_font_name: &'a mut String,
    selected_font_path: &'a mut String,
    reload_font_clicked: &'a mut bool,

    selected_log_dir_path: &'a mut String,
    reload_log_clicked: &'a mut bool,

    selected_i18n_path: &'a mut String,
    reload_i18n_clicked: &'a mut bool,
    write_i18n_clicked: &'a mut bool,
}

struct PathSelector<'a> {
    ui: &'a mut egui::Ui,
    label: &'a str,
    value: &'a mut String,
    select_label: &'a str,
    reload_label: &'a str,
    reload_hover: &'a str,
    clear_label: &'a str,
}

/// Return reload button clicked
fn path_selector_row(selector: PathSelector, picker: impl FnOnce() -> Option<String>) -> bool {
    let PathSelector { ui, label, value, select_label, reload_label, reload_hover, clear_label } =
        selector;

    ui.label(label);

    let mut reload_clicked = false;

    ui.horizontal(|ui| {
        ui.text_edit_singleline(value);

        if ui.button(select_label).clicked()
            && let Some(p) = picker()
        {
            *value = p;
        }

        if ui.button(reload_label).on_hover_text(reload_hover).clicked() {
            reload_clicked = true;
        }

        if ui.button(clear_label).clicked() {
            value.clear();
        }
    });
    ui.end_row();

    reload_clicked
}
