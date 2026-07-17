//! Single toast notification entry.

/// A single toast message.
#[derive(Clone)]
pub struct ToastEntry {
    pub title: String,
    pub description: Option<String>,
    pub variant: crate::tokens::toast_variant::ToastVariant,
    pub created_at: f64,
    pub duration_secs: f64,
}
