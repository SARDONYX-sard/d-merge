//! Separator builder struct — a 1px line divider.

/// A horizontal or vertical separator line.
#[must_use]
pub struct Separator {
    pub(crate) horizontal: bool,
    pub(crate) text: Option<String>,
}

impl Separator {
    pub fn horizontal() -> Self {
        Self { horizontal: true, text: None }
    }

    pub fn vertical() -> Self {
        Self { horizontal: false, text: None }
    }

    /// Adds a centered text label to the separator.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
