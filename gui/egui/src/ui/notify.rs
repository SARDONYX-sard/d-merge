//! Notification bar

use egui::{Color32, Visuals};

/// Color specification for a notification.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) enum NotificationColor {
    #[default]
    ThemeText,
    ThemeSuccess,
    ThemeInfo,
    ThemeWarning,
    ThemeError,
    Custom(Color32),
}

impl NotificationColor {
    pub(crate) const RIGHT_BLUE: egui::Color32 = egui::Color32::from_rgb(120, 220, 255);

    /// Resolves the notification color using the current theme.
    #[inline]
    pub(crate) fn resolve(self, visuals: &Visuals) -> Color32 {
        match self {
            Self::ThemeText => visuals.text_color(),
            Self::ThemeSuccess => egui::Color32::from_rgb(100, 200, 120), // Color32::GREEN,  Not found success color
            Self::ThemeInfo => Self::RIGHT_BLUE,
            Self::ThemeWarning => visuals.warn_fg_color,
            Self::ThemeError => visuals.error_fg_color,
            Self::Custom(color) => color,
        }
    }
}

/// Notification message shown in the bottom bar.
#[derive(Debug, Clone, Default)]
pub(crate) struct Notification {
    pub message: String,
    pub color: NotificationColor,
}

impl Notification {
    /// Sets a success message.
    #[inline]
    pub(crate) fn success(&mut self, msg: impl Into<String>) {
        self.set_with_color(msg, NotificationColor::ThemeSuccess);
    }

    /// Sets an informational message.
    #[inline]
    pub(crate) fn info(&mut self, msg: impl Into<String>) {
        self.set_with_color(msg, NotificationColor::ThemeInfo);
    }

    /// Sets a warning message.
    #[inline]
    pub(crate) fn warn(&mut self, msg: impl Into<String>) {
        self.set_with_color(msg, NotificationColor::ThemeWarning);
    }

    /// Sets an error message.
    #[inline]
    pub(crate) fn error(&mut self, msg: impl Into<String>) {
        self.set_with_color(msg, NotificationColor::ThemeError);
    }

    /// Sets a message with an explicit color.
    #[inline]
    pub(crate) fn set(&mut self, msg: impl Into<String>, color: Color32) {
        self.set_with_color(msg, NotificationColor::Custom(color));
    }

    /// Sets a message with a color specification.
    #[inline]
    fn set_with_color(&mut self, msg: impl Into<String>, color: NotificationColor) {
        self.message = msg.into();
        self.color = color;
    }

    /// Clears the current message.
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.message.clear();
    }
}
