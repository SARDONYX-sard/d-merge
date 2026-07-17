//! shadcn-styled Radio builder struct.

/// A radio button widget styled after shadcn/ui.
#[must_use]
pub struct Radio<'a> {
    pub(crate) selected: &'a mut bool,
    pub(crate) label: Option<egui::WidgetText>,
}

impl<'a> Radio<'a> {
    pub fn new(selected: &'a mut bool) -> Self {
        Self {
            selected,
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
