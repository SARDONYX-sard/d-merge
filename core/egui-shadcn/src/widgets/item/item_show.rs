//! Show method for Item — renders a bordered list item.

impl super::item::Item {
    /// Renders the item container with content.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let cr = theme.radius.round() as u8;

        let border_color = match self.variant {
            crate::tokens::item_variant::ItemVariant::Outline => theme.border,
            crate::tokens::item_variant::ItemVariant::Default => egui::Color32::TRANSPARENT,
        };

        let frame = egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: 12,  // px-3
                right: 12,
                top: 10,   // py-2.5
                bottom: 10,
            })
            .corner_radius(egui::CornerRadius::same(cr))
            .stroke(egui::Stroke::new(1.0, border_color));

        frame.show(ui, content).response
    }
}
