//! AlertDialog builder struct — a confirmation modal.

/// A confirmation dialog with cancel and action buttons.
#[must_use]
pub struct AlertDialog {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) cancel_text: String,
    pub(crate) action_text: String,
    pub(crate) destructive: bool,
}

impl AlertDialog {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            cancel_text: "Cancel".to_owned(),
            action_text: "Continue".to_owned(),
            destructive: false,
        }
    }

    pub fn cancel_text(mut self, text: impl Into<String>) -> Self {
        self.cancel_text = text.into();
        self
    }

    pub fn action_text(mut self, text: impl Into<String>) -> Self {
        self.action_text = text.into();
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }
}
