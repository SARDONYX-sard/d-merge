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

use std::path::Path;

use d_merge_gui_shared::i18n::{I18nKey, I18nMap};
use egui::Color32;

use crate::{app::App, ui::confirm::ConfirmAction};

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
        let clear_button_name = self.t(I18nKey::ClearButton).to_string();

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
    /// Anchored to the viewport centre.  All mutations (issue link open,
    /// path changes, i18n reload) are deferred to after the closure returns
    /// to satisfy the borrow checker.
    pub(crate) fn ui_help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }

        fn path_selector_row(
            ui: &mut egui::Ui,
            label: &str,
            value: &mut String,
            select_label: &str,
            clear_label: &str,
            hover: Option<&str>,
            picker: impl FnOnce() -> Option<String>,
        ) {
            ui.horizontal(|ui| {
                let r = ui.label(label);
                if let Some(h) = hover {
                    r.on_hover_text(h);
                }

                ui.text_edit_singleline(value);

                if ui.button(select_label).clicked()
                    && let Some(p) = picker()
                {
                    *value = p;
                }

                if ui.button(clear_label).clicked() {
                    value.clear();
                }
            });
        }

        let window_title = self.t(I18nKey::HelpButton).to_string();

        let issue_report_label = self.t(I18nKey::IssueReportButton).to_string();
        let issue_report_hover = self.t(I18nKey::IssueReportHover).to_string();

        let select_button = self.t(I18nKey::SelectButton).to_string();
        let clear_button = self.t(I18nKey::ClearButton).to_string();

        let bug_report_label = self.t(I18nKey::BugReportLabel).to_string();
        let see_issues_label = self.t(I18nKey::BugReportSeeIssues).to_string();
        let tooling_label = self.t(I18nKey::ToolingLabel).to_string();

        let log_dir_label = self.t(I18nKey::LogDirPathLabel).to_string();
        let i18n_path_label = self.t(I18nKey::I18nPathLabel).to_string();
        let restart_note = self.t(I18nKey::RestartRequiredNote).to_string();

        let i18n_write_label = self.t(I18nKey::I18nWriteNewJsonButton).to_string();
        let i18n_write_hover = format!(
            "{} (-> {})",
            self.t(I18nKey::I18nWriteNewJsonHover),
            self.settings.ui.i18n_path,
        );

        let i18n_reload_label = self.t(I18nKey::I18nReloadJsonButton).to_string();
        let i18n_reload_hover = format!(
            "{} (-> {})",
            self.t(I18nKey::I18nReloadJsonHover),
            self.settings.ui.i18n_path,
        );

        let issue_url = self.settings.create_issue_link();

        let mut show = true;
        let mut issue_clicked = false;
        let mut write_i18n_clicked = false;
        let mut reload_i18n_clicked = false;

        let mut selected_log_dir_path = self.settings.log.dir_path.clone();
        let mut selected_i18n_path = self.settings.ui.i18n_path.clone();

        egui::Window::new(window_title)
            .open(&mut show)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(crate::APP_TITLE);
                });

                ui.add_space(8.0);

                ui.separator();

                let rows = [
                    (self.t(I18nKey::AuthorLabel).to_string(), env!("CARGO_PKG_AUTHORS"), None),
                    (
                        self.t(I18nKey::LicenseLabel).to_string(),
                        env!("CARGO_PKG_LICENSE"),
                        Some(concat!(
                            env!("CARGO_PKG_REPOSITORY"),
                            "/blob/",
                            env!("CARGO_PKG_VERSION"),
                            "/LICENSE"
                        )),
                    ),
                    (
                        self.t(I18nKey::SourceCodeLabel).to_string(),
                        "GitHub",
                        Some(concat!(
                            env!("CARGO_PKG_REPOSITORY"),
                            "/tree/",
                            env!("CARGO_PKG_VERSION")
                        )),
                    ),
                    (
                        self.t(I18nKey::ChangeLogLabel).to_string(),
                        "CHANGELOG.md",
                        Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/CHANGELOG.md")),
                    ),
                    (
                        self.t(I18nKey::ModTestStatusLabel).to_string(),
                        "test_status.md",
                        Some(concat!(
                            env!("CARGO_PKG_REPOSITORY"),
                            "/blob/",
                            env!("CARGO_PKG_VERSION"),
                            "/docs/test_status.md"
                        )),
                    ),
                ];

                egui::Grid::new("help_info_grid").num_columns(2).spacing([8.0, 6.0]).show(
                    ui,
                    |ui| {
                        for (l, v, url) in rows {
                            ui.label(l);
                            match url {
                                Some(url) => ui.hyperlink_to(v, url).on_hover_text(url),
                                None => ui.label(v),
                            };

                            ui.end_row();
                        }
                    },
                );

                ui.add_space(8.0);

                ui.separator();
                ui.add_space(4.0);

                ui.label(&bug_report_label);

                ui.horizontal(|ui| {
                    const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");

                    ui.hyperlink_to(&see_issues_label, ISSUE_URL).on_hover_text(ISSUE_URL);

                    if ui.button(&issue_report_label).on_hover_text(&issue_report_hover).clicked() {
                        issue_clicked = true;
                    }
                });

                ui.add_space(8.0);

                ui.separator();
                ui.add_space(4.0);

                ui.label(&tooling_label);

                ui.add_space(4.0);

                path_selector_row(
                    ui,
                    &log_dir_label,
                    &mut selected_log_dir_path,
                    &select_button,
                    &clear_button,
                    Some(&restart_note),
                    || {
                        let p = Path::new(self.settings.log.dir_path.as_str());
                        rfd::FileDialog::new()
                            .set_directory(p)
                            .pick_folder()
                            .map(|p| p.display().to_string())
                    },
                );

                ui.add_space(4.0);

                path_selector_row(
                    ui,
                    &i18n_path_label,
                    &mut selected_i18n_path,
                    &select_button,
                    &clear_button,
                    None,
                    || {
                        let p = Path::new(&self.settings.ui.i18n_path);
                        rfd::FileDialog::new()
                            .set_directory(p)
                            .add_filter("translation.json", &["json"])
                            .pick_file()
                            .map(|p| p.display().to_string())
                    },
                );

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button(&i18n_write_label).on_hover_text(&i18n_write_hover).clicked() {
                        write_i18n_clicked = true;
                    }

                    if ui.button(&i18n_reload_label).on_hover_text(&i18n_reload_hover).clicked() {
                        reload_i18n_clicked = true;
                    }
                });
            });

        if !show {
            self.show_help = false;
        }

        if issue_clicked {
            ctx.open_url(egui::OpenUrl { url: issue_url, new_tab: true });
        }

        if self.settings.log.dir_path != selected_log_dir_path {
            self.settings.log.dir_path = selected_log_dir_path;
            self.set_colored_notify(
                "Log dir path updated. Restart the app to apply changes.".to_string(),
                Color32::YELLOW,
            );
        }

        if self.settings.ui.i18n_path != selected_i18n_path {
            self.settings.ui.i18n_path = selected_i18n_path;
        }

        if write_i18n_clicked {
            self.confirm_dialog.open(i18n_write_hover, ConfirmAction::WriteI18nJson);
        }

        if reload_i18n_clicked {
            self.reload_i18n();
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

    /// Renders the confirmation dialog and dispatches the confirmed action.
    pub(crate) fn ui_show_confirm(&mut self, ctx: &egui::Context) {
        let confirmed_action = {
            let mut result = None;
            self.confirm_dialog.show(ctx, |action| result = Some(action));
            result
        };

        if let Some(action) = confirmed_action {
            match action {
                ConfirmAction::WriteI18nJson => {
                    match I18nMap::save(self.settings.ui.i18n_path.as_str()) {
                        Ok(()) => self.set_colored_notify(
                            format!("OK. Wrote {}", self.settings.ui.i18n_path),
                            Color32::GREEN,
                        ),
                        Err(err) => self.notify_error(err.to_string()),
                    }
                }
            }
        }
    }
}
