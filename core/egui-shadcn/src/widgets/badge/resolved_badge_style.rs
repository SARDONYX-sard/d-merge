//! Resolved concrete style values for a Badge.

/// Fully resolved colors for painting a badge.
pub struct ResolvedBadgeStyle {
    pub bg: egui::Color32,
    pub fg: egui::Color32,
    pub border: Option<egui::Color32>,
}
