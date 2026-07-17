//! Resolved concrete style values for a Select.

/// Fully resolved colors for painting a select trigger and dropdown.
pub struct ResolvedSelectStyle {
    pub trigger_bg: egui::Color32,
    pub trigger_border: egui::Color32,
    pub trigger_text: egui::Color32,
    pub popover_bg: egui::Color32,
    pub popover_border: egui::Color32,
    pub item_text: egui::Color32,
    pub item_hover_bg: egui::Color32,
    pub corner_radius: f32,
}
