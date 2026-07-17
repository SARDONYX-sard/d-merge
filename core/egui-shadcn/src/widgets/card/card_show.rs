//! Show method for Card — renders content inside a bordered container.

impl super::card::Card {
    /// Renders the card container and calls `content` inside it.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let ring_color = egui::Color32::from_rgba_unmultiplied(
            theme.foreground.r(),
            theme.foreground.g(),
            theme.foreground.b(),
            26, // ~10% of foreground
        );
        let cr = theme.radius + 2.0; // rounded-xl = radius + 2

        let frame = egui::Frame::NONE
            .fill(theme.card)
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 16,
                bottom: 16,
            })
            .corner_radius(egui::CornerRadius::same(cr.round() as u8))
            .stroke(egui::Stroke::new(1.0, ring_color));

        frame.show(ui, content).response
    }
}
