//! NavigationMenu builder struct — a horizontal navigation bar.

/// A navigation menu: horizontal bar of clickable links.
#[must_use]
pub struct NavigationMenu {
    pub(crate) items: Vec<String>,
}

impl NavigationMenu {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }
}
