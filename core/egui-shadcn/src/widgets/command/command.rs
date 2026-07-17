//! Command builder struct — a command palette with search.

/// A command palette: centered modal with search input and command list.
#[must_use]
pub struct Command {
    pub(crate) items: Vec<(String, String)>,
    pub(crate) placeholder: String,
}

impl Command {
    /// Items are `(group_name, command_label)` pairs.
    pub fn new(items: Vec<(String, String)>) -> Self {
        Self {
            items,
            placeholder: "Type a command or search...".to_owned(),
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }
}
