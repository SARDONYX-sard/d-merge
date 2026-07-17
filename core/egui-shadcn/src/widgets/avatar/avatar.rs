//! Avatar builder struct — circle with initials fallback.

/// An avatar circle: `size-8 rounded-full bg-muted`.
#[must_use]
pub struct Avatar {
    pub(crate) initials: String,
    pub(crate) size: f32,
}

impl Avatar {
    pub fn new(initials: impl Into<String>) -> Self {
        Self {
            initials: initials.into(),
            size: 32.0, // size-8
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
