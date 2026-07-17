//! shadcn-styled Checkbox builder struct.

/// A checkbox widget styled after shadcn/ui.
#[must_use]
pub struct Checkbox<'a> {
    pub(crate) checked: &'a mut bool,
    pub(crate) label: Option<egui::WidgetText>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool) -> Self {
        Self {
            checked,
            label: None,
        }
    }

    pub fn label(mut self, label: impl Into<egui::WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
