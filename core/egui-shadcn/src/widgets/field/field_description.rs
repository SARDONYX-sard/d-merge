//! FieldDescription — `text-muted-foreground text-sm` helper text.

/// A field description in muted text.
pub struct FieldDescription;

impl FieldDescription {
    /// Paints a description line below a field.
    pub fn show(ui: &mut egui::Ui, text: &str) {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(14.0), // text-sm
            theme.muted_foreground,
        );
        let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().galley(rect.min, galley, theme.muted_foreground);
        }
    }
}
