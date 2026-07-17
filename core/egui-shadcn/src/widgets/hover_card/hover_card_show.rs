//! Show method for HoverCard — renders a popup on hover.

impl super::hover_card::HoverCard {
    /// Shows a hover card when `trigger_response` is hovered.
    pub fn show(self, trigger_response: &egui::Response, content: impl FnOnce(&mut egui::Ui)) {
        let theme =
            crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(&trigger_response.ctx);
        let cr = (theme.radius + 2.0).round() as u8;

        let themed_frame = egui::Frame::NONE
            .fill(theme.popover)
            .inner_margin(egui::Margin::same(16))
            .corner_radius(egui::CornerRadius::same(cr))
            .stroke(egui::Stroke::new(1.0, theme.border))
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(8),
            });

        let mut tooltip = egui::Tooltip::for_enabled(trigger_response);
        tooltip.popup = tooltip.popup.at_pointer().gap(12.0).frame(themed_frame);

        tooltip.show(|ui| {
            ui.style_mut().visuals.override_text_color = Some(theme.popover_foreground);
            ui.set_min_width(self.width);
            content(ui);
        });
    }
}
