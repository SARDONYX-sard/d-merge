//! Tooltip builder struct — a hover popup with styled text.

/// A tooltip popup: `bg-primary text-primary-foreground rounded-md px-3 py-1.5 text-xs`.
#[must_use]
pub struct Tooltip {
    pub(crate) text: String,
}

impl Tooltip {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}
