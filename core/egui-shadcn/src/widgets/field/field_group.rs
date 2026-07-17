//! FieldGroup — vertical stack with gap-5 (20px) spacing.

/// A vertical stack of form fields with consistent spacing.
pub struct FieldGroup;

impl FieldGroup {
    /// Renders child content in a vertical layout with 20px gaps.
    pub fn show(
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::InnerResponse<()> {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 20.0; // gap-5
            content(ui);
        })
    }
}
