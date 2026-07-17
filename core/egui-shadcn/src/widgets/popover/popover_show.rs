//! Show method for Popover — renders a popup anchored to a trigger response.

impl super::popover::Popover {
    /// Shows a popover anchored to `trigger_response`. Click to toggle.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        trigger_response: &egui::Response,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let popup_id = trigger_response.id.with("popover");

        let toggle_cmd = if trigger_response.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else {
            None
        };

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

        let popup = egui::Popup::new(
            popup_id,
            ui.ctx().clone(),
            trigger_response,
            ui.layer_id(),
        )
        .open_memory(toggle_cmd)
        .frame(themed_frame);

        popup.show(|ui: &mut egui::Ui| {
            ui.set_min_width(200.0);
            content(ui);
        });
    }
}
