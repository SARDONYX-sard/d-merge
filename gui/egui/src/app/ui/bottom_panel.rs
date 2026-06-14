//! Bottom-panel UI: notification bar, action buttons, log-level selector.
//!
//! Two [`egui::TopBottomPanel::bottom`] panels are rendered:
//!
//! | id                   | content                                      |
//! |----------------------|----------------------------------------------|
//! | `notification_panel` | Single-line coloured status message          |
//! | `bottom_panel`       | Log dir · Log viewer · Patch · Help buttons  |
//!
//! `notification_panel` is registered first so it sits below `bottom_panel`
//! visually (egui stacks bottom panels innermost-first).

use d_merge_gui_shared::{
    fetch::FetchState, fs::open_existing_dir_or_ancestor, i18n::I18nKey, log::LogLevel,
};

use crate::app::App;

impl App {
    /// Renders the main bottom panel (log controls, patch button, help toggle).
    ///
    /// The patch button is disabled while a fetch is in progress to prevent
    /// launching a patch against a stale mod list.
    pub(crate) fn ui_bottom_panel(&mut self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::bottom("bottom_panel");
        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }

        panel.show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.ui_log_level_box(ui);

                self.add_button(ui, ctx, I18nKey::LogDir, |s, _| {
                    if let Err(err) =
                        open_existing_dir_or_ancestor(s.settings.log.dir_path.as_str())
                    {
                        s.notify_error(err);
                    }
                });
                self.add_button(ui, ctx, I18nKey::LogButton, |s, _| {
                    use std::sync::atomic::Ordering;
                    s.show_log_window.fetch_xor(true, Ordering::Relaxed);
                });
                self.add_button(ui, ctx, I18nKey::NotificationClearButton, |s, _| {
                    s.clear_notification();
                });

                let is_fetching = matches!(*self.fetch_state.read(), FetchState::Fetching);
                ui.add_enabled_ui(!is_fetching, |ui| {
                    let label = if is_fetching {
                        self.i18n.t(I18nKey::PatchFetchingButton)
                    } else {
                        self.i18n.t(I18nKey::PatchButton)
                    };
                    if ui.add_sized([120.0, 40.0], egui::Button::new(label)).clicked() {
                        self.patch(ui.ctx());
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [30.0, 40.0],
                            egui::Button::new(self.i18n.t(I18nKey::HelpButton)),
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
    fn add_button<F>(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, key: I18nKey, f: F)
    where
        F: FnOnce(&mut Self, &egui::Context),
    {
        if ui.add_sized([120.0, 40.0], egui::Button::new(self.i18n.t(key))).clicked() {
            f(self, ctx);
        }
    }

    /// Renders the log-level combo box.
    ///
    /// Changing the level takes effect immediately by calling
    /// `tracing_rotation::global::change_level`.
    fn ui_log_level_box(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(self.i18n.t(I18nKey::LogLevelLabel));

            egui::ComboBox::from_id_salt("log_level")
                .selected_text(self.settings.log.level.as_str())
                .show_ui(ui, |ui| {
                    for level in [
                        LogLevel::Error,
                        LogLevel::Warn,
                        LogLevel::Info,
                        LogLevel::Debug,
                        LogLevel::Trace,
                    ] {
                        if ui
                            .selectable_value(&mut self.settings.log.level, level, level.as_str())
                            .changed()
                        {
                            tracing_rotation::global::change_level(level.as_str()).unwrap();
                        }
                    }
                });
        });
    }

    /// Renders the single-line notification bar at the very bottom.
    ///
    /// The message and color are set by [`App::set_colored_notify`] and
    /// cleared by [`App::clear_notification`].
    pub(crate) fn ui_notification(&self, ctx: &egui::Context) {
        let mut panel = egui::TopBottomPanel::bottom("notification_panel");
        if self.settings.ui.transparent {
            panel = panel.frame(egui::Frame::new());
        }
        panel.show(ctx, |ui| {
            ui.colored_label(self.notify.1, &self.notify.0);
        });
    }
}
