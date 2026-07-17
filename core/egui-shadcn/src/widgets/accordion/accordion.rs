//! Accordion builder struct -- multiple collapsible sections.

/// An accordion: `divide-y` sections with toggleable content.
#[must_use]
pub struct Accordion {
    pub(crate) items: Vec<(String, String)>,
    pub(crate) multiple: bool,
}

impl Accordion {
    /// Creates a new accordion. Items are `(title, content)` pairs.
    pub fn new(items: Vec<(String, String)>) -> Self {
        Self {
            items,
            multiple: false,
        }
    }

    /// Allows multiple sections to be open simultaneously.
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }
}
