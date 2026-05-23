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

use egui::Color32;

use crate::{
    app::App,
    i18n::{I18nKey, I18nMap},
    ui::confirm::ConfirmAction,
};

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
                let response = ui.label(label);

                if let Some(hover) = hover {
                    response.on_hover_text(hover);
                }

                ui.text_edit_singleline(value);

                if ui.button(select_label).clicked()
                    && let Some(path) = picker()
                {
                    *value = path;
                }

                if ui.button(clear_label).clicked() {
                    value.clear();
                }
            });
        }

        // ── Pre-extract strings ───────────────────────────────────────────────
        let window_title = self.t(I18nKey::HelpButton).to_string();

        let issue_report_label = self.t(I18nKey::IssueReportButton).to_string();
        let issue_report_hover = self.t(I18nKey::IssueReportHover).to_string();

        let select_button = self.t(I18nKey::SelectButton).to_string();
        let clear_button = "Clear".to_string();

        let i18n_write_label = self.t(I18nKey::I18nWriteNewJsonButton).to_string();

        let i18n_write_hover = format!(
            "{} (path: {})",
            self.t(I18nKey::I18nWriteNewJsonHover),
            self.settings.ui.i18n_path,
        );

        let i18n_reload_label = self.t(I18nKey::I18nReloadJsonButton).to_string();

        let i18n_reload_hover = format!(
            "{} (path: {})",
            self.t(I18nKey::I18nReloadJsonHover),
            self.settings.ui.i18n_path,
        );

        let issue_url = self.settings.create_issue_link();

        // ── Deferred mutations ────────────────────────────────────────────────
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
                // ── App info ──────────────────────────────────────────────────
                ui.vertical_centered(|ui| {
                    ui.heading(crate::APP_TITLE);
                });

                ui.add_space(8.0);

                // ── Links grid ───────────────────────────────────────────────
                ui.separator();
                ui.add_space(4.0);

                const LICENSE_URL: &str = concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/blob/",
                    env!("CARGO_PKG_VERSION"),
                    "/LICENSE",
                );

                const SOURCE_CODE_URL: &str =
                    concat!(env!("CARGO_PKG_REPOSITORY"), "/tree/", env!("CARGO_PKG_VERSION"),);

                const CHANGELOG_URL: &str =
                    concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/CHANGELOG.md");

                const MOD_TEST_URL: &str = concat!(
                    env!("CARGO_PKG_REPOSITORY"),
                    "/blob/",
                    env!("CARGO_PKG_VERSION"),
                    "/docs/test_status.md",
                );

                let rows = [
                    ("Author:", env!("CARGO_PKG_AUTHORS"), None),
                    ("License:", env!("CARGO_PKG_LICENSE"), Some(LICENSE_URL)),
                    ("Source Code:", "GitHub", Some(SOURCE_CODE_URL)),
                    ("Change Log:", "CHANGELOG.md", Some(CHANGELOG_URL)),
                    ("Mod Test Status:", "test_status.md", Some(MOD_TEST_URL)),
                ];

                egui::Grid::new("help_info_grid").num_columns(2).spacing([8.0, 6.0]).show(
                    ui,
                    |ui| {
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
                    },
                );

                ui.add_space(8.0);

                // ── Bug report ────────────────────────────────────────────────
                ui.separator();
                ui.add_space(4.0);

                ui.label("Bug Report:");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");

                    ui.hyperlink_to("See Issues", ISSUE_URL).on_hover_text(ISSUE_URL);

                    if ui.button(&issue_report_label).on_hover_text(&issue_report_hover).clicked() {
                        issue_clicked = true;
                    }
                });

                ui.add_space(8.0);

                // ── Tooling ───────────────────────────────────────────────────
                ui.separator();
                ui.add_space(4.0);

                ui.label("Tooling:");
                ui.add_space(4.0);

                path_selector_row(
                    ui,
                    "Log dir path:",
                    &mut selected_log_dir_path,
                    &select_button,
                    &clear_button,
                    Some("NOTE: You'll need to restart the app for changes to take effect."),
                    || {
                        let p = Path::new(self.settings.log.dir_path.as_str());

                        match p.canonicalize() {
                            Ok(cp) => rfd::FileDialog::new().set_directory(cp),
                            Err(_) => rfd::FileDialog::new(),
                        }
                        .pick_folder()
                        .map(|p| p.display().to_string())
                    },
                );

                ui.add_space(4.0);

                path_selector_row(
                    ui,
                    "I18n Path:",
                    &mut selected_i18n_path,
                    &select_button,
                    &clear_button,
                    None,
                    || {
                        let p = Path::new(&self.settings.ui.i18n_path);

                        match p.parent() {
                            Some(parent) if parent.exists() => {
                                rfd::FileDialog::new().set_directory(parent)
                            }
                            _ => rfd::FileDialog::new(),
                        }
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

        // ── Apply deferred mutations ──────────────────────────────────────────
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
