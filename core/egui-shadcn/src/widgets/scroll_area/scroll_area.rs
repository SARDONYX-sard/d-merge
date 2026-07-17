//! ScrollArea builder struct — a themed scrollable container.

/// A styled scroll area wrapping egui's built-in ScrollArea.
#[must_use]
pub struct ScrollArea {
    pub(crate) max_height: f32,
    pub(crate) horizontal: bool,
}

impl ScrollArea {
    pub fn new(max_height: f32) -> Self {
        Self {
            max_height,
            horizontal: false,
        }
    }

    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
}
