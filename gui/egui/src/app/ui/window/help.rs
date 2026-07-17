//! help dialog, confirmation dialog.

use std::{borrow::Cow, path::Path};

use d_merge_gui_shared::{
    fs::open_existing_dir_or_ancestor,
    i18n::{I18nKey, I18nMap},
    settings::ui::FontMode,
};
use egui::Color32;

use crate::{
    app::App,
    ui::shadcn_compat::{button, checkbox, enum_select, heading, searchable_string_select},
};

bitflags::bitflags! {
    /// Actions requested by the widgets inside the help dialog during a single frame.
    ///
    /// Each section function (`ui_font_section`, `ui_log_section`, ...) only ever
    /// receives a `&mut HelpDialogActions` instead of a handful of separate `bool`s,
    /// and sets the relevant bits once it knows what happened. `ui_help_window`
    /// drains this single value after the window closes and dispatches side effects.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    struct HelpDialogActions: u32 {
        /// "Reload" button clicked in the font section.
        const FONT_RELOAD_CLICKED        = 1 << 0;
        /// A new font file was picked via the `rfd` file dialog.
        const FONT_PATH_SELECTED         = 1 << 1;

        /// "Reload" button clicked in the log section.
        const LOG_RELOAD_CLICKED         = 1 << 2;
        /// A new log directory was picked via the `rfd` file dialog.
        const LOG_PATH_SELECTED          = 1 << 3;

        /// "Reload" button clicked in the i18n section.
        const I18N_RELOAD_CLICKED        = 1 << 4;
        /// A new translation file was picked via the `rfd` file dialog.
        const I18N_PATH_SELECTED         = 1 << 5;
        /// "Write new translation.json" button clicked.
        const I18N_WRITE_CLICKED         = 1 << 6;
        /// "Default" (force English) button clicked.
        const I18N_LOAD_DEFAULT_CLICKED  = 1 << 7;

        /// "Reload" button clicked in the background section.
        const BACKGROUND_RELOAD_CLICKED  = 1 << 8;
        /// A new background image was picked via the `rfd` file dialog.
        const BACKGROUND_PATH_SELECTED   = 1 << 9;
    }
}

impl App {
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

        let mut selected_log_dir_path = self.settings.log.dir_path.clone();

        let mut selected_i18n_path = self.settings.ui.i18n_path.clone();

        let mut selected_background_path = self.settings.ui.background.path.clone();
        let mut background_enabled = self.settings.ui.background.enabled;

        // Single carrier for every widget-triggered action in this frame.
        let mut actions = HelpDialogActions::empty();

        egui::Window::new(self.i18n.t(I18nKey::HelpButton))
            .open(&mut show_help)
            .collapsible(false)
            .resizable(true)
            .movable(true)
            .fixed_size(egui::Vec2::new(
                self.settings.ui.window.width * 0.5,
                self.settings.ui.window.height * 0.75,
            ))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(heading(crate::APP_TITLE));
                });

                ui.add_space(8.0);
                ui.separator();

                self.ui_help_info(ui);
                ui.separator();

                self.ui_bug_report(ui);
                ui.separator();

                self.ui_font_section(
                    ui,
                    &mut selected_font_mode,
                    &mut selected_font_name,
                    &mut selected_font_path,
                    &mut actions,
                );
                ui.separator();

                self.ui_log_section(ui, &mut selected_log_dir_path, &mut actions);
                ui.separator();

                self.ui_translation_section(ui, &mut selected_i18n_path, &mut actions);
                ui.separator();

                self.ui_background_section(
                    ui,
                    &mut selected_background_path,
                    &mut background_enabled,
                    &mut actions,
                );
                ui.separator();

                // NOTE: Since it shrinks automatically, extend it to the `height`. (To create space for potential font heights.)
                ui.add_space(ui.available_height());
            });

        if !show_help {
            self.show_help = false;
        }

        // --- Font actions ---
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
        // NOTE: The font path won't be applied automatically until we click the reload button
        // or actually pick a new file. (Because auto-applying on every keystroke would make it
        // harder to use.)
        if ((did_font_mode_change
            && matches!(selected_font_mode, FontMode::Default | FontMode::System))
            || did_font_name_change
            || actions.intersects(
                HelpDialogActions::FONT_RELOAD_CLICKED | HelpDialogActions::FONT_PATH_SELECTED,
            ))
            && let Err(err) = crate::fonts::set_fonts(ctx, &self.settings.ui.font)
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

        // --- Log actions ---
        if self.settings.log.dir_path != selected_log_dir_path {
            self.settings.log.dir_path = selected_log_dir_path;
        }
        if actions.intersects(
            HelpDialogActions::LOG_RELOAD_CLICKED | HelpDialogActions::LOG_PATH_SELECTED,
        ) {
            self.reload_log();
        }

        // --- i18n actions ---
        if self.settings.ui.i18n_path != selected_i18n_path {
            self.settings.ui.i18n_path = selected_i18n_path;
        }
        if actions.intersects(
            HelpDialogActions::I18N_RELOAD_CLICKED | HelpDialogActions::I18N_PATH_SELECTED,
        ) {
            self.reload_i18n();
        }
        if actions.contains(HelpDialogActions::I18N_LOAD_DEFAULT_CLICKED) {
            self.i18n = d_merge_gui_shared::i18n::I18nMap::new();
        }
        if actions.contains(HelpDialogActions::I18N_WRITE_CLICKED) {
            self.write_new_i18n();
        }

        // --- Background actions ---
        if self.settings.ui.background.path != selected_background_path {
            self.settings.ui.background.path = selected_background_path;
        }
        self.settings.ui.background.enabled = background_enabled;
        if actions.intersects(
            HelpDialogActions::BACKGROUND_RELOAD_CLICKED
                | HelpDialogActions::BACKGROUND_PATH_SELECTED,
        ) {
            self.settings.ui.background.enabled = true;
            self.reload_background(ctx);
        }
    }

    fn ui_help_info(&self, ui: &mut egui::Ui) {
        let rows = [
            (self.i18n.t(I18nKey::AuthorLabel), env!("CARGO_PKG_AUTHORS"), None),
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
            (
                self.i18n.t(I18nKey::SourceCodeLabel),
                "GitHub",
                Some(concat!(env!("CARGO_PKG_REPOSITORY"), "/tree/", env!("CARGO_PKG_VERSION"))),
            ),
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
        ];

        egui::Grid::new("help_info_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
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
        let bug_report_label = self.i18n.t(I18nKey::BugReportLabel);

        let see_issues_label = self.i18n.t(I18nKey::BugReportSeeIssues);

        let issue_report_label = self.i18n.t(I18nKey::IssueReportButton);
        let issue_report_hover = self.i18n.t(I18nKey::IssueReportHover);

        ui.label(bug_report_label);

        ui.horizontal(|ui| {
            if ui.add(button(issue_report_label)).on_hover_text(issue_report_hover).clicked() {
                ui.ctx().open_url(egui::OpenUrl {
                    url: self.settings.create_issue_link(),
                    new_tab: true,
                });
            }

            const ISSUE_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/issues");
            ui.hyperlink_to(see_issues_label, ISSUE_URL).on_hover_text(ISSUE_URL);
        });
    }

    fn ui_font_section(
        &self,
        ui: &mut egui::Ui,
        selected_font_mode: &mut FontMode,
        selected_font_name: &mut String,
        selected_font_path: &mut String,
        actions: &mut HelpDialogActions,
    ) {
        egui::Grid::new("font_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label(self.i18n.t(I18nKey::FontModeLabel))
                .on_hover_text(self.i18n.t(I18nKey::FontModeHover));
            enum_select(
                ui,
                selected_font_mode,
                &[
                    (FontMode::Default, self.i18n.t(I18nKey::FontModeDefault)),
                    (FontMode::System, self.i18n.t(I18nKey::FontModeSystem)),
                    (FontMode::File, self.i18n.t(I18nKey::FontModeFile)),
                ],
                None::<egui::Vec2>,
            );
            ui.end_row();
            match selected_font_mode {
                FontMode::Default => {}

                FontMode::System => {
                    ui.label(self.i18n.t(I18nKey::FontFamily));

                    searchable_string_select(
                        ui,
                        "font_family",
                        selected_font_name,
                        crate::fonts::font_families(),
                        format!("{}...", self.i18n.t(I18nKey::SearchLabel)),
                    );

                    ui.end_row();
                }

                FontMode::File => {
                    // Local flags for this section only; folded into `actions` at the end.
                    let mut path_selected = false;

                    let reload_clicked = path_selector_row(
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
                                .add_filter("font", &["ttc", "ttf", "tto"])
                                .pick_file()
                                .map(|p| {
                                    path_selected = true;
                                    p.display().to_string()
                                })
                        },
                    );

                    actions.set(HelpDialogActions::FONT_RELOAD_CLICKED, reload_clicked);
                    actions.set(HelpDialogActions::FONT_PATH_SELECTED, path_selected);
                }
            }
            ui.end_row();
        });
    }

    fn ui_log_section(
        &self,
        ui: &mut egui::Ui,
        selected_log_dir_path: &mut String,
        actions: &mut HelpDialogActions,
    ) {
        egui::Grid::new("log_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            let mut path_selected = false;

            let reload_clicked = path_selector_row(
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
                    let dir = dir.canonicalize().map(Cow::Owned).unwrap_or(Cow::Borrowed(dir));

                    rfd::FileDialog::new().set_directory(dir).pick_folder().map(|p| {
                        path_selected = true;
                        p.display().to_string()
                    })
                },
            );

            actions.set(HelpDialogActions::LOG_RELOAD_CLICKED, reload_clicked);
            actions.set(HelpDialogActions::LOG_PATH_SELECTED, path_selected);

            ui.end_row();
        });
    }

    fn ui_translation_section(
        &self,
        ui: &mut egui::Ui,
        selected_i18n_path: &mut String,
        actions: &mut HelpDialogActions,
    ) {
        egui::Grid::new("i18n_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            let mut path_selected = false;

            let reload_clicked = path_selector_row(
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
                    let path = Path::new(&self.settings.ui.i18n_path).parent().map_or_else(
                        || Cow::Borrowed(Path::new(".")),
                        |p| p.canonicalize().map_or(Cow::Borrowed(p), Cow::Owned),
                    );

                    rfd::FileDialog::new()
                        .set_directory(path)
                        .add_filter("translation", &["json"])
                        .pick_file()
                        .map(|p| {
                            path_selected = true;
                            p.display().to_string()
                        })
                },
            );

            actions.set(HelpDialogActions::I18N_RELOAD_CLICKED, reload_clicked);
            actions.set(HelpDialogActions::I18N_PATH_SELECTED, path_selected);

            ui.label("English:");
            ui.horizontal(|ui| {
                if ui
                    .add(button(self.i18n.t(I18nKey::I18nWriteNewJsonButton)))
                    .on_hover_text(self.i18n.t(I18nKey::I18nWriteNewJsonHover))
                    .clicked()
                {
                    actions.insert(HelpDialogActions::I18N_WRITE_CLICKED);
                }

                if ui
                    .add(button("Default"))
                    .on_hover_text("Temporarily force a switch to English mode (for debugging)")
                    .clicked()
                {
                    actions.insert(HelpDialogActions::I18N_LOAD_DEFAULT_CLICKED);
                }
            });

            ui.end_row();
        });
    }

    fn ui_background_section(
        &self,
        ui: &mut egui::Ui,
        selected_background_path: &mut String,
        background_enabled: &mut bool,
        actions: &mut HelpDialogActions,
    ) {
        egui::Grid::new("background_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            let mut path_selected = false;

            let reload_clicked = path_selector_row(
                PathSelector {
                    ui,
                    label: self.i18n.t(I18nKey::BackgroundImageLabel),
                    value: selected_background_path,
                    select_label: self.i18n.t(I18nKey::SelectButton),
                    reload_label: self.i18n.t(I18nKey::ReloadButton),
                    reload_hover: self.i18n.t(I18nKey::BackgroundImageReloadHover),
                    clear_label: self.i18n.t(I18nKey::ClearButton),
                },
                || {
                    let dir = Path::new(&self.settings.ui.background.path).parent().map_or_else(
                        || Cow::Borrowed(Path::new(".")),
                        |p| p.canonicalize().map_or(Cow::Borrowed(p), Cow::Owned),
                    );

                    rfd::FileDialog::new()
                        .set_directory(dir)
                        .add_filter("image", &["png", "jpg", "jpeg"])
                        .pick_file()
                        .map(|p| {
                            path_selected = true;
                            p.display().to_string()
                        })
                },
            );
            actions.set(HelpDialogActions::BACKGROUND_RELOAD_CLICKED, reload_clicked);
            actions.set(HelpDialogActions::BACKGROUND_PATH_SELECTED, path_selected);

            checkbox(ui, background_enabled, self.i18n.t(I18nKey::BackgroundImageEnabled))
                .on_hover_text(self.i18n.t(I18nKey::BackgroundImageEnabledHover));
            ui.end_row();
        });
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

    fn write_new_i18n(&mut self) {
        let path = Path::new(&self.settings.ui.i18n_path).parent().map_or_else(
            || Cow::Borrowed(Path::new(".")),
            |p| p.canonicalize().map_or(Cow::Borrowed(p), Cow::Owned),
        );

        let path = rfd::FileDialog::new()
            .set_directory(path)
            .set_title("Save translation.json")
            .set_file_name("translation.json")
            .add_filter("translation", &["json"])
            .save_file();

        if let Some(path) = path {
            match I18nMap::save(&path) {
                Ok(()) => {
                    self.set_colored_notify(
                        format!("OK. Wrote {}", path.display()),
                        Color32::GREEN,
                    );
                }
                Err(err) => self.notify_error(err.to_string()),
            }
        }
    }
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

/// Return whether the "reload" button was clicked.
fn path_selector_row(selector: PathSelector, picker: impl FnOnce() -> Option<String>) -> bool {
    let PathSelector { ui, label, value, select_label, reload_label, reload_hover, clear_label } =
        selector;

    if ui.add(button(label)).on_hover_text(value.as_str()).clicked()
        && let Err(err) = open_existing_dir_or_ancestor(std::path::Path::new(value))
    {
        tracing::error!(err);
    }

    let mut reload_clicked = false;

    ui.horizontal(|ui| {
        ui.text_edit_singleline(value);

        if ui.add(button(select_label)).clicked()
            && let Some(p) = picker()
        {
            *value = p;
        }

        if ui.add(button(reload_label)).on_hover_text(reload_hover).clicked() {
            reload_clicked = true;
        }

        if ui.add(button(clear_label)).clicked() {
            value.clear();
        }
    });
    ui.end_row();

    reload_clicked
}
