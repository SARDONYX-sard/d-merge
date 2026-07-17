//! Textarea builder struct — a multi-line text input styled after shadcn/ui.

/// A multi-line text area: `border-input rounded-lg px-2.5 py-2 min-h-16`.
#[must_use]
pub struct Textarea<'a> {
    pub(crate) text: &'a mut String,
    pub(crate) placeholder: String,
    pub(crate) desired_width: Option<f32>,
    pub(crate) min_height: f32,
}

impl<'a> Textarea<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: String::new(),
            desired_width: None,
            min_height: 64.0, // min-h-16 = 4rem = 64px
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

    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = height;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
