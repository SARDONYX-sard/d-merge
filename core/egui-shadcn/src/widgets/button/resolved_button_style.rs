//! Resolved concrete style values for a Button in a given state.

/// Fully resolved colors and metrics for painting a button.
pub struct ResolvedButtonStyle {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub border: Option<egui::Color32>,
    pub corner_radius: f32,
    pub height: f32,
    pub h_padding: f32,
    pub font_size: f32,
    pub underline: bool,
}
