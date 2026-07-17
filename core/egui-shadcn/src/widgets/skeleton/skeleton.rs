//! Skeleton builder struct — an animated loading placeholder.

/// A skeleton placeholder: `animate-pulse rounded-md bg-primary/10`.
#[must_use]
pub struct Skeleton {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) circle: bool,
}

impl Skeleton {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            circle: false,
        }
    }

    /// Makes the skeleton a circle (uses height as diameter).
    pub fn circle(mut self) -> Self {
        self.circle = true;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
