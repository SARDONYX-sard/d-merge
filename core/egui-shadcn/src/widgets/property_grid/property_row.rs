//! PropertyRow builder struct.

/// A single label/control row for an inspector or settings panel.
#[must_use]
pub struct PropertyRow {
    pub(crate) label: String,
    pub(crate) label_width: Option<f32>,
}

impl PropertyRow {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            label_width: None,
        }
    }

    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = Some(width);
        self
    }
}
