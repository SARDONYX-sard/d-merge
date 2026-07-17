//! Alert visual variants.

/// Visual variants for the Alert component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertVariant {
    /// Standard informational alert with default colors.
    #[default]
    Default,
    /// Red destructive tint for error or warning messages.
    Destructive,
}
