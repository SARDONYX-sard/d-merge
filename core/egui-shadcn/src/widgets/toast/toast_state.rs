//! Toast notification state manager.

/// Manages active toast notifications.
#[derive(Default, Clone)]
pub struct ToastState {
    pub(crate) toasts: Vec<super::toast_entry::ToastEntry>,
}

impl ToastState {
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    /// Adds a toast notification. Uses context time for creation timestamp.
    pub fn add(
        &mut self,
        title: impl Into<String>,
        variant: crate::tokens::toast_variant::ToastVariant,
        time: f64,
    ) {
        self.toasts.push(super::toast_entry::ToastEntry {
            title: title.into(),
            description: None,
            variant,
            created_at: time,
            duration_secs: 4.0,
        });
    }

    /// Adds a toast with description.
    pub fn add_with_description(
        &mut self,
        title: impl Into<String>,
        description: impl Into<String>,
        variant: crate::tokens::toast_variant::ToastVariant,
        time: f64,
    ) {
        self.toasts.push(super::toast_entry::ToastEntry {
            title: title.into(),
            description: Some(description.into()),
            variant,
            created_at: time,
            duration_secs: 4.0,
        });
    }

    /// Removes expired toasts.
    pub fn cleanup(&mut self, current_time: f64) {
        self.toasts
            .retain(|t| current_time - t.created_at < t.duration_secs);
    }
}
