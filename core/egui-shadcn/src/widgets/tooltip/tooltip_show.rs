//! Show method for Tooltip — wraps a response with a styled tooltip on hover.

impl super::tooltip::Tooltip {
    /// Attaches a tooltip to the given response. Call after the trigger widget.
    pub fn show(self, response: &egui::Response) {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(&response.ctx);

        let themed_frame = egui::Frame::NONE
            .fill(theme.primary)
            .inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 6,
                bottom: 6,
            })
            .corner_radius(egui::CornerRadius::same(6));

        let mut tooltip = egui::Tooltip::for_enabled(response);
        tooltip.popup = tooltip.popup.frame(themed_frame);

        tooltip.show(|ui| {
            ui.label(
                egui::RichText::new(&self.text)
                    .color(theme.primary_foreground)
                    .size(12.0),
            );
        });
    }
}
