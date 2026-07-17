//! ToggleGroup builder struct — a set of exclusive toggle buttons.

/// A group of toggle buttons: `inline-flex gap-0.5 rounded-lg bg-muted p-0.5`.
#[must_use]
pub struct ToggleGroup {
    pub(crate) items: Vec<String>,
    pub(crate) variant: crate::tokens::toggle_variant::ToggleVariant,
}

impl ToggleGroup {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            variant: crate::tokens::toggle_variant::ToggleVariant::Default,
        }
    }

    pub fn variant(mut self, variant: crate::tokens::toggle_variant::ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}
