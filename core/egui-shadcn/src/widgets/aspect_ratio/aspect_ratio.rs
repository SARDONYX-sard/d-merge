//! AspectRatio builder struct — a container that enforces a width:height ratio.

/// A container that constrains content to a specific aspect ratio.
#[must_use]
pub struct AspectRatio {
    pub(crate) ratio: f32,
}

impl AspectRatio {
    /// Creates a new aspect ratio container. `ratio` is width/height (e.g. 16.0/9.0).
    pub fn new(ratio: f32) -> Self {
        Self { ratio }
    }
}
