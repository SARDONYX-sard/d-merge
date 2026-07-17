//! Progress builder struct -- a horizontal progress bar.

/// A progress bar: `h-2 rounded-full bg-primary/20`.
#[must_use]
pub struct Progress {
    pub(crate) value: f32,
}

impl Progress {
    /// Creates a new progress bar. `value` is clamped to 0.0..=1.0.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
