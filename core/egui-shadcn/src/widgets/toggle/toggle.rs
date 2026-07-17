//! shadcn-styled Toggle builder struct.

/// A toggle button that maintains pressed/unpressed state.
#[must_use]
pub struct Toggle<'a> {
    pub(crate) pressed: &'a mut bool,
    pub(crate) text: egui::WidgetText,
    pub(crate) variant: crate::tokens::toggle_variant::ToggleVariant,
    pub(crate) size: crate::tokens::component_size::ComponentSize,
}

impl<'a> Toggle<'a> {
    pub fn new(pressed: &'a mut bool, text: impl Into<egui::WidgetText>) -> Self {
        Self {
            pressed,
            text: text.into(),
            variant: crate::tokens::toggle_variant::ToggleVariant::Default,
            size: crate::tokens::component_size::ComponentSize::Default,
        }
    }

    pub fn variant(mut self, variant: crate::tokens::toggle_variant::ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: crate::tokens::component_size::ComponentSize) -> Self {
        self.size = size;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
