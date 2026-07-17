//! Breadcrumb builder struct — a navigation path indicator.

/// A breadcrumb navigation: items separated by `/` or `>`.
#[must_use]
pub struct Breadcrumb {
    pub(crate) items: Vec<String>,
    pub(crate) separator: String,
}

impl Breadcrumb {
    /// Creates a new breadcrumb from a list of path items.
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            separator: "/".to_owned(),
        }
    }

    /// Sets the separator string between breadcrumb items (default: `/`).
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }
}
