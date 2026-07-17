//! Resolved concrete style values for a Slider.

/// Fully resolved colors for painting a slider.
pub struct ResolvedSliderStyle {
    pub track_color: egui::Color32,
    pub fill_color: egui::Color32,
    pub handle_fill: egui::Color32,
    pub handle_border: egui::Color32,
}
