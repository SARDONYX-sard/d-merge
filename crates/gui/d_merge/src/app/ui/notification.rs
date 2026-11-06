//! The notification bar, under every page.

use crate::app::App;

impl App {
    /// Renders the single-line notification bar at the very bottom.
    ///
    /// It is shared by all pages.
    pub(crate) fn ui_notification(&self, ui: &mut egui::Ui) {
        let panel = egui::Panel::bottom("notification_panel").frame(self.panel_frame());

        panel.show(ui, |ui| {
            ui.colored_label(self.notify.color.resolve(ui.visuals()), &self.notify.message);
        });
    }
}
