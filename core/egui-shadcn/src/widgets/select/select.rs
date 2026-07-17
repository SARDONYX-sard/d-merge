//! shadcn-styled Select builder struct.

/// A dropdown select widget styled after shadcn/ui.
///
/// `T` must be cloneable, displayable, and comparable. The widget takes a mutable
/// reference to the currently selected value and a slice of options.
#[must_use]
pub struct Select<'a, T: Clone + std::fmt::Display + PartialEq + 'static> {
    pub(crate) selected: &'a mut Option<T>,
    pub(crate) options: &'a [T],
    pub(crate) placeholder: String,
    pub(crate) width: Option<f32>,
    pub(crate) selected_text_override: Option<String>,
}

impl<'a, T: Clone + std::fmt::Display + PartialEq + 'static> Select<'a, T> {
    pub fn new(selected: &'a mut Option<T>, options: &'a [T]) -> Self {
        Self {
            selected,
            options,
            placeholder: "Select...".to_owned(),
            width: None,
            selected_text_override: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the display text shown in the trigger (instead of `T::to_string()`).
    pub fn selected_text(mut self, text: impl Into<String>) -> Self {
        self.selected_text_override = Some(text.into());
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}

/// Non-Option variant: takes `&mut T` directly (always has a value selected).
#[must_use]
pub struct SelectValue<'a, T: Clone + std::fmt::Display + PartialEq + 'static> {
    pub(crate) selected: &'a mut T,
    pub(crate) options: &'a [T],
    pub(crate) width: Option<f32>,
    pub(crate) selected_text_override: Option<String>,
}

impl<'a, T: Clone + std::fmt::Display + PartialEq + 'static> SelectValue<'a, T> {
    pub fn new(selected: &'a mut T, options: &'a [T]) -> Self {
        Self {
            selected,
            options,
            width: None,
            selected_text_override: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Override the display text shown in the trigger.
    pub fn selected_text(mut self, text: impl Into<String>) -> Self {
        self.selected_text_override = Some(text.into());
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        ui.add(self)
    }
}
