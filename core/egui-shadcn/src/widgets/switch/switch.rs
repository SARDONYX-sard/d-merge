//! shadcn-styled Switch builder struct.

/// A toggle switch widget styled after shadcn/ui.
#[must_use]
pub struct Switch<'a> {
    pub(crate) on: &'a mut bool,
    pub(crate) label: Option<egui::WidgetText>,
}

impl<'a> Switch<'a> {
    pub fn new(on: &'a mut bool) -> Self {
        Self { on, label: None }
    }

    pub fn label(mut self, label: impl Into<egui::WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
