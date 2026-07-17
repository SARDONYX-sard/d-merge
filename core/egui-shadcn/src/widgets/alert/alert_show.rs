//! Show method for Alert — renders a status message container.

impl super::alert::Alert {
    /// Renders the alert and calls `content` for the description area.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let style = super::alert_variant_style::resolve_alert_style(&theme, self.variant);
        let cr = egui::CornerRadius::same(theme.radius.round() as u8);

        let frame = egui::Frame::NONE
            .fill(style.bg)
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 12,
                bottom: 12,
            })
            .corner_radius(cr)
            .stroke(egui::Stroke::new(1.0, style.border));

        frame
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if let Some(title) = self.title {
                        ui.label(
                            egui::RichText::new(title)
                                .color(style.fg)
                                .size(14.0)
                                .strong(),
                        );
                        ui.add_space(2.0);
                    }
                    ui.scope(|ui| {
                        ui.style_mut().visuals.override_text_color = Some(style.fg);
                        content(ui);
                    });
                });
            })
            .response
    }
}
