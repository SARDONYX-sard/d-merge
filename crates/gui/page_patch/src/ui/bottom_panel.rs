//! The panel under the mod list: log controls and the patch button.

use egui_widgets::shadcn::{Button, ComboBox, Icon};
use gui_shared::{fs::open_existing_dir_or_ancestor, i18n::I18nKey, log::LogLevel};

use super::PatchView;

impl PatchView<'_> {
    /// Renders the main bottom panel (log controls, patch button).
    ///
    /// The patch button is disabled while a fetch is in progress to prevent
    /// launching a patch against a stale mod list.
    pub(super) fn ui_bottom_panel(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let panel = egui::Panel::bottom("bottom_panel").frame(self.ctx.panel_frame);

        panel.show(ui, |ui| {
            ui.horizontal(|ui| {
                self.ui_log_level_box(ui);

                self.add_button(ui, ctx, I18nKey::LogDir, |app, _| {
                    if let Err(err) = open_existing_dir_or_ancestor(app.ctx.log.dir_path.as_str()) {
                        app.ctx.notify.error(err);
                    }
                });
                self.add_button(ui, ctx, I18nKey::LogButton, |app, _| {
                    use std::sync::atomic::Ordering;
                    app.ctx.show_log_window.fetch_xor(true, Ordering::Relaxed);
                });
                self.add_button(ui, ctx, I18nKey::NotificationClearButton, |app, _| {
                    app.ctx.notify.clear();
                });

                let is_fetching = self.page.fetch.is_fetching();
                // A check is a merge too, and there is one at a time.
                ui.add_enabled_ui(!is_fetching && !self.page.conflicts.is_scanning(), |ui| {
                    let label = if is_fetching {
                        self.ctx.i18n.t(I18nKey::PatchFetchingButton)
                    } else {
                        self.ctx.i18n.t(I18nKey::PatchButton)
                    };
                    if ui
                        .add_sized([120.0, 40.0], Button::new(label).icon(Icon::Layers).full_width().primary())
                        .clicked()
                    {
                        self.page.run.start(
                            ui.ctx(),
                            self.ctx.settings,
                            self.ctx.notify,
                            self.ctx.i18n,
                            self.ctx.runtime,
                        );
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| self.pandora_button(ui));
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
        if ui.add_sized([120.0, 40.0], Button::new(self.ctx.i18n.t(key)).icon(Icon::Logs).full_width()).clicked() {
            f(self, ctx);
        }
    }

    /// Renders the log-level combo box.
    ///
    /// Changing the level takes effect immediately by calling
    /// `tracing_rotation::global::change_level`.
    fn ui_log_level_box(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(self.ctx.i18n.t(I18nKey::LogLevelLabel));

            const LEVELS: [(LogLevel, &str); 5] = [
                (LogLevel::Error, LogLevel::Error.as_str()),
                (LogLevel::Warn, LogLevel::Warn.as_str()),
                (LogLevel::Info, LogLevel::Info.as_str()),
                (LogLevel::Debug, LogLevel::Debug.as_str()),
                (LogLevel::Trace, LogLevel::Trace.as_str()),
            ];

            let mut index = LEVELS.iter().position(|(level, _)| *level == self.ctx.log.level).unwrap_or(0);
            if ComboBox::from_id_salt("log_level")
                .width(120.0)
                .show_index(ui, &mut index, LEVELS.len(), |i| LEVELS[i].1)
                .changed()
            {
                self.ctx.log.level = LEVELS[index].0;
                let _ = tracing_rotation::global::change_level(self.ctx.log.level.as_str());
            }
        });
    }
}
