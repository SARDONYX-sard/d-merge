//! Resolved concrete style values for a Switch.

/// Fully resolved colors for painting a switch.
pub struct ResolvedSwitchStyle {
    pub track_color: egui::Color32,
    pub track_border: Option<egui::Color32>,
    pub thumb_color: egui::Color32,
}
