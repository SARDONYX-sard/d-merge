//! Collapsible builder struct — a panel with toggle trigger.

/// A collapsible panel with a trigger header.
#[must_use]
pub struct Collapsible {
    pub(crate) title: String,
}

impl Collapsible {
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into() }
    }
}
