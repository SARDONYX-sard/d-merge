//! Color swatch builder struct.

/// A clickable color swatch with optional label and hex readout.
#[must_use]
pub struct ColorSwatch {
    pub(crate) color: egui::Color32,
    pub(crate) label: Option<String>,
    pub(crate) selected: bool,
    pub(crate) size: f32,
    pub(crate) show_hex: bool,
}

impl ColorSwatch {
    pub fn new(color: egui::Color32) -> Self {
        Self {
            color,
            label: None,
            selected: false,
            size: 28.0,
            show_hex: false,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn show_hex(mut self) -> Self {
        self.show_hex = true;
        self
    }
}
