//! Button variant enum matching shadcn/ui button variants.

/// Visual variants for the Button widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Primary background with primary foreground text.
    #[default]
    Default,
    /// Red/destructive background.
    Destructive,
    /// Border only, transparent background.
    Outline,
    /// Secondary (muted) background.
    Secondary,
    /// Fully transparent, colored on hover.
    Ghost,
    /// Text-only with underline on hover.
    Link,
}
