//! Notification bar

use egui::Color32;

use crate::app::App;

impl App {
    /// Sets the notification message with the default (white) color.
    #[inline]
    pub(crate) fn notify_info<S: Into<String>>(&mut self, msg: S) {
        self.set_colored_notify(msg, Color32::WHITE);
    }

    /// Sets the notification message with the default (white) color.
    #[inline]
    pub(crate) fn notify_error<S: Into<String>>(&mut self, msg: S) {
        self.set_colored_notify(msg, egui::Color32::RED);
    }

    /// Sets the notification message with an explicit color.
    ///
    /// Typical usage:
    /// ```ignore
    /// self.set_colored_notify("Done", Color32::GREEN);
    /// self.notify_error(err);
    /// ```
    pub(crate) fn set_colored_notify<S>(&mut self, msg: S, color: Color32)
    where
        S: Into<String>,
    {
        self.notify = (msg.into(), color);
    }

    /// Clears the current notification message.
    pub(crate) fn clear_notification(&mut self) {
        self.notify.0.clear();
    }
}
