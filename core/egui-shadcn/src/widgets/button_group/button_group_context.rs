//! Temporary context stored in egui data during ButtonGroup rendering.

/// Context stored in egui temp data while a ButtonGroup is being rendered.
/// Buttons check the `active` flag to adjust their corner radii and borders.
#[derive(Clone, Default)]
pub struct ButtonGroupContext {
    /// Whether a button group is currently being rendered.
    pub active: bool,
    /// Right-edge x positions of each button in the group.
    pub boundaries: Vec<f32>,
    /// How many buttons were in the group last frame (for first/last detection).
    pub cached_count: usize,
    /// Current button index, incremented by each button.
    pub current_index: usize,
    /// The group's corner radius.
    pub corner_radius: f32,
    /// Union rect of all buttons in the group so far.
    pub group_rect: Option<egui::Rect>,
}
