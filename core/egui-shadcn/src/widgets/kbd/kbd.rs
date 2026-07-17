//! Kbd builder struct — a keyboard shortcut badge.

/// A keyboard shortcut display: `rounded-md border bg-muted px-1.5 text-xs font-mono`.
#[must_use]
pub struct Kbd {
    pub(crate) text: String,
}

impl Kbd {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
