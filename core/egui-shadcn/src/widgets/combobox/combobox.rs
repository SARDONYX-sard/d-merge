//! Combobox builder struct — a searchable dropdown select.

/// A combobox: input with dropdown filter list.
#[must_use]
pub struct Combobox {
    pub(crate) items: Vec<String>,
    pub(crate) placeholder: String,
    pub(crate) width: Option<f32>,
}

impl Combobox {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            placeholder: "Select...".to_owned(),
            width: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
}
