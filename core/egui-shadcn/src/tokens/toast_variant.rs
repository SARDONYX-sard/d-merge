//! Toast visual variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastVariant {
    #[default]
    Default,
    Success,
    Error,
    Warning,
    Info,
}
