//! Show method for StatusBar.

impl super::status_bar::StatusBar {
    /// Renders a compact status bar and calls `content` inside it.
    pub fn show(self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let margin = if self.dense {
            egui::Margin {
                left: 8,
                right: 8,
                top: 4,
                bottom: 4,
            }
        } else {
            egui::Margin {
                left: 10,
                right: 10,
                top: 6,
                bottom: 6,
            }
        };

        let frame = egui::Frame::NONE
            .fill(theme.muted)
            .inner_margin(margin)
            .corner_radius(egui::CornerRadius::same((theme.radius * 0.75).round() as u8))
            .stroke(egui::Stroke::new(1.0, theme.border));

        frame
            .show(ui, |ui| {
                ui.horizontal_wrapped(content);
            })
            .response
    }
}
