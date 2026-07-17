//! Show method for Toolbar.

impl super::toolbar::Toolbar {
    /// Renders a compact command bar and calls `content` inside it.
    pub fn show(self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let margin = if self.dense {
            egui::Margin {
                left: 6,
                right: 6,
                top: 5,
                bottom: 5,
            }
        } else {
            egui::Margin {
                left: 8,
                right: 8,
                top: 7,
                bottom: 7,
            }
        };

        let frame = egui::Frame::NONE
            .fill(theme.card)
            .inner_margin(margin)
            .corner_radius(egui::CornerRadius::same(theme.radius.round() as u8))
            .stroke(egui::Stroke::new(1.0, theme.border));

        frame
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = self.spacing;
                ui.spacing_mut().item_spacing.y = self.spacing;
                if self.wrap {
                    ui.horizontal_wrapped(content);
                } else {
                    ui.horizontal(content);
                }
            })
            .response
    }
}
