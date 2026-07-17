//! Resolved concrete style values for an Alert.

/// Fully resolved colors for painting an alert.
pub struct ResolvedAlertStyle {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub border: egui::Color32,
}
