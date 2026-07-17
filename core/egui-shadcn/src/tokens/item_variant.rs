//! Item style variants.

/// Visual variants for the Item component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemVariant {
    /// Transparent border.
    #[default]
    Default,
    /// Visible border.
    Outline,
}
