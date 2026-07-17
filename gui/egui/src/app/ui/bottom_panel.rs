//! Bottom-panel UI: notification bar, action buttons, log-level selector.

use d_merge_gui_shared::{
    fetch::FetchState, fs::open_existing_dir_or_ancestor, i18n::I18nKey, log::LogLevel,
};

use crate::{
    app::App,
    theme::themed_top_bottom_panel,
    ui::shadcn_compat::{button_with_icon, enum_select, patch_button},
};

impl App {
    /// Renders the main bottom panel (log controls, patch button, help toggle).
    ///
    /// The patch button is disabled while a fetch is in progress to prevent
    /// launching a patch against a stale mod list.
    pub(crate) fn ui_bottom_panel(&mut self, ui: &mut egui::Ui) {
        let panel = themed_top_bottom_panel(
            egui::Panel::bottom("bottom_panel"),
            self.settings.ui.theme,
            self.theme_manager.current_bg_color(),
        );

        panel.show(ui, |ui| {
            ui.horizontal(|ui| {
                self.ui_log_level_box(ui);

                self.add_button(ui, I18nKey::LogDir, |s, _| {
                    if let Err(err) =
                        open_existing_dir_or_ancestor(s.settings.log.dir_path.as_str())
                    {
                        s.notify_error(err);
                    }
                });
                self.add_button(ui, I18nKey::LogButton, |s, _| {
                    use std::sync::atomic::Ordering;
                    s.show_log_window.fetch_xor(true, Ordering::Relaxed);
                });
                self.add_button(ui, I18nKey::NotificationClearButton, |s, _| {
                    s.clear_notification();
                });

                let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);
                ui.add_enabled_ui(!is_fetching, |ui| {
                    let label = if is_fetching {
                        self.i18n.t(I18nKey::PatchFetchingButton)
                    } else {
                        self.i18n.t(I18nKey::PatchButton)
                    };
                    if ui.add_sized([120.0, 40.0], patch_button(label)).clicked() {
                        self.patch(ui.ctx());
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [100.0, 40.0],
                            button_with_icon(
                                self.i18n.t(I18nKey::HelpButton),
                                egui_shadcn::LucideIcon::Toolbox,
                            ),
                        )
                        .clicked()
                    {
                        self.show_help ^= true;
                    }
                });
            });
        });
    }

    /// Convenience helper: renders a fixed-size button and calls `f` on click.
    ///
    /// Reduces boilerplate in [`ui_bottom_panel`] where every button follows
    /// the same `add_sized` + `clicked` + closure pattern.
    fn add_button<F>(&mut self, ui: &mut egui::Ui, key: I18nKey, f: F)
    where
        F: FnOnce(&mut Self, &egui::Context),
    {
        if ui
            .add_sized(
                [120.0, 40.0],
                button_with_icon(self.i18n.t(key), egui_shadcn::LucideIcon::Logs),
            )
            .clicked()
        {
            f(self, ui.ctx());
        }
    }

    /// Renders the log-level combo box.
    ///
    /// Changing the level takes effect immediately by calling
    /// `tracing_rotation::global::change_level`.
    fn ui_log_level_box(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(self.i18n.t(I18nKey::LogLevelLabel));

            const LEVELS: [(LogLevel, &str); 5] = [
                (LogLevel::Error, LogLevel::Error.as_str()),
                (LogLevel::Warn, LogLevel::Warn.as_str()),
                (LogLevel::Info, LogLevel::Info.as_str()),
                (LogLevel::Debug, LogLevel::Debug.as_str()),
                (LogLevel::Trace, LogLevel::Trace.as_str()),
            ];

            if enum_select(ui, &mut self.settings.log.level, &LEVELS, Some([120.0, 30.0])).changed()
            {
                let _ = tracing_rotation::global::change_level(self.settings.log.level.as_str());
            }
        });
    }

    /// Renders the single-line notification bar at the very bottom.
    ///
    /// The message and color are set by [`App::set_colored_notify`] and
    /// cleared by [`App::clear_notification`].
    pub(crate) fn ui_notification(&self, ui: &mut egui::Ui) {
        let panel = themed_top_bottom_panel(
            egui::Panel::bottom("notification_panel"),
            self.settings.ui.theme,
            self.theme_manager.current_bg_color(),
        );

        panel.show(ui, |ui| {
            ui.colored_label(self.notify.1, &self.notify.0);
        });
    }
}
