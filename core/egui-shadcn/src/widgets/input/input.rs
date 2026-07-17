//! shadcn-styled Input builder struct.

/// A text input field styled after shadcn/ui.
#[must_use]
pub struct Input<'a> {
    pub(crate) text: &'a mut String,
    pub(crate) placeholder: String,
    pub(crate) desired_width: Option<f32>,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: String::new(),
            desired_width: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn desired_width(mut self, width: f32) -> Self {
        self.desired_width = Some(width);
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
