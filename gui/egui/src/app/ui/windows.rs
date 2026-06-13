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
    /// Anchored to the viewport centre.  All mutations (issue link open,
    /// path changes, i18n reload) are deferred to after the closure returns
    /// to satisfy the borrow checker.
    pub(crate) fn ui_help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }

        let mut show = true;

        let mut issue_clicked = false;
        let mut write_i18n_clicked = false;
        let mut reload_i18n_clicked = false;

        let mut selected_log_dir_path = self.settings.log.dir_path.clone();
        let mut selected_i18n_path = self.settings.ui.i18n_path.clone();

        let issue_url = self.settings.create_issue_link();

        egui::Window::new(self.i18n.t(I18nKey::HelpButton))
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

                self.ui_help_info(ui);

                ui.add_space(8.0);
                ui.separator();

                self.ui_bug_report(ui, &mut issue_clicked);

                ui.add_space(8.0);
                ui.separator();

                self.ui_tooling(
                    ui,
                    &mut selected_log_dir_path,
                    &mut selected_i18n_path,
                    &mut write_i18n_clicked,
                    &mut reload_i18n_clicked,
                );
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
            self.confirm_dialog.open(
                format!(
                    "{} (-> {})",
                    self.i18n.t(I18nKey::I18nWriteNewJsonHover),
                    self.settings.ui.i18n_path,
                ),
                ConfirmAction::WriteI18nJson,
            );
        }

        if reload_i18n_clicked {
            self.reload_i18n();
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

    fn ui_bug_report(&self, ui: &mut egui::Ui, issue_clicked: &mut bool) {
        let bug_report_label = self.i18n.t(I18nKey::BugReportLabel);
        let see_issues_label = self.i18n.t(I18nKey::BugReportSeeIssues);

        let issue_report_label = self.i18n.t(I18nKey::IssueReportButton);
        let issue_report_hover = self.i18n.t(I18nKey::IssueReportHover);

        ui.label(bug_report_label);

        ui.horizontal(|ui| {
            const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");

            ui.hyperlink_to(see_issues_label, ISSUE_URL).on_hover_text(ISSUE_URL);

            if ui.button(issue_report_label).on_hover_text(issue_report_hover).clicked() {
                *issue_clicked = true;
            }
        });
    }

    fn ui_tooling(
        &self,
        ui: &mut egui::Ui,
        selected_log_dir_path: &mut String,
        selected_i18n_path: &mut String,
        write_i18n_clicked: &mut bool,
        reload_i18n_clicked: &mut bool,
    ) {
        let tooling_label = self.i18n.t(I18nKey::ToolingLabel);

        let select_button = self.i18n.t(I18nKey::SelectButton);
        let clear_button = self.i18n.t(I18nKey::ClearButton);

        let log_dir_label = self.i18n.t(I18nKey::LogDirPathLabel);

        let i18n_path_label = self.i18n.t(I18nKey::I18nPathLabel);
        let i18n_write_label = self.i18n.t(I18nKey::I18nWriteNewJsonButton);
        let i18n_reload_label = self.i18n.t(I18nKey::I18nReloadJsonButton);
        let restart_note = self.i18n.t(I18nKey::RestartRequiredNote);

        let i18n_write_hover = format!(
            "{} (-> {})",
            self.i18n.t(I18nKey::I18nWriteNewJsonHover),
            self.settings.ui.i18n_path,
        );

        let i18n_reload_hover = format!(
            "{} (-> {})",
            self.i18n.t(I18nKey::I18nReloadJsonHover),
            self.settings.ui.i18n_path,
        );

        ui.label(tooling_label);

        ui.add_space(4.0);

        egui::Grid::new("tooling_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            path_selector_row(
                ui,
                log_dir_label,
                selected_log_dir_path,
                select_button,
                clear_button,
                Some(restart_note),
                || {
                    let dir = Path::new(self.settings.log.dir_path.as_str());

                    rfd::FileDialog::new()
                        .set_directory(dir)
                        .pick_folder()
                        .map(|p| p.display().to_string())
                },
            );

            path_selector_row(
                ui,
                i18n_path_label,
                selected_i18n_path,
                select_button,
                clear_button,
                None,
                || {
                    let dir = Path::new(&self.settings.ui.i18n_path);

                    rfd::FileDialog::new()
                        .set_directory(dir)
                        .add_filter("translation.json", &["json"])
                        .pick_file()
                        .map(|p| p.display().to_string())
                },
            );
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button(i18n_write_label).on_hover_text(&i18n_write_hover).clicked() {
                *write_i18n_clicked = true;
            }

            if ui.button(i18n_reload_label).on_hover_text(&i18n_reload_hover).clicked() {
                *reload_i18n_clicked = true;
            }
        });
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

fn path_selector_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    select_label: &str,
    clear_label: &str,
    hover: Option<&str>,
    picker: impl FnOnce() -> Option<String>,
) {
    let r = ui.label(label);

    if let Some(h) = hover {
        r.on_hover_text(h);
    }

    ui.horizontal(|ui| {
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

    ui.end_row();
}
