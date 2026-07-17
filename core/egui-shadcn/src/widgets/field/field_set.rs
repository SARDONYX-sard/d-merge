//! FieldSet — group of fields with legend and description.

/// A group of fields with an optional legend heading.
pub struct FieldSet;

impl FieldSet {
    /// Renders a fieldset with a legend title and content.
    pub fn show(
        ui: &mut egui::Ui,
        legend: &str,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::InnerResponse<()> {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            super::field_legend::FieldLegend::show(ui, legend);
            content(ui);
        })
    }
}
