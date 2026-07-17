//! Show method for ScrollArea — renders a themed scrollable region.

impl super::scroll_area::ScrollArea {
    pub fn show(
        self,
        ui: &mut egui::Ui,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let cr = egui::CornerRadius::same(theme.radius.round() as u8);

        let frame = egui::Frame::NONE
            .fill(egui::Color32::TRANSPARENT)
            .corner_radius(cr)
            .stroke(egui::Stroke::new(1.0, theme.border));

        frame
            .show(ui, |ui| {
                if self.horizontal {
                    egui::ScrollArea::horizontal()
                        .max_height(self.max_height)
                        .show(ui, content);
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(self.max_height)
                        .show(ui, content);
                }
            })
            .response
    }
}
