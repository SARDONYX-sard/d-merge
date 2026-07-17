//! Toggle variant enum.

/// Visual variants for the Toggle widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleVariant {
    /// Default toggle style.
    #[default]
    Default,
    /// Outlined toggle with border.
    Outline,
}
