//! ButtonGroup builder — connected button strip with merged borders.

/// Position within a button group, used to adjust corner radii.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonGroupPosition {
    First,
    Middle,
    Last,
    Only,
}

/// A horizontal group of connected buttons.
pub struct ButtonGroup;
