//! Resolved concrete style values for an Input.

/// Fully resolved colors for painting an input field.
pub struct ResolvedInputStyle {
    pub bg: egui::Color32,
    pub border_color: egui::Color32,
    pub text_color: egui::Color32,
    pub placeholder_color: egui::Color32,
    pub corner_radius: f32,
}
