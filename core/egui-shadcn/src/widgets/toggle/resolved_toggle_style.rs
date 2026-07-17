//! Resolved concrete style values for a Toggle.

/// Fully resolved colors for painting a toggle button.
pub struct ResolvedToggleStyle {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub border: Option<egui::Color32>,
    pub corner_radius: f32,
}
