//! Show method for PropertyGrid.

impl super::property_grid::PropertyGrid {
    /// Renders a bordered property grid and calls `content` inside it.
    pub fn show(self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let key = super::property_grid_context_key::property_grid_context_key();

        let previous = ui.ctx().data_mut(|data| {
            let previous = data.get_temp::<super::property_grid_context::PropertyGridContext>(key);
            data.insert_temp(
                key,
                super::property_grid_context::PropertyGridContext {
                    label_width: self.label_width,
                },
            );
            previous
        });

        let frame = egui::Frame::NONE
            .fill(theme.card)
            .inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 10,
                bottom: 10,
            })
            .corner_radius(egui::CornerRadius::same(theme.radius.round() as u8))
            .stroke(egui::Stroke::new(1.0, theme.border));

        let response = frame
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = self.row_gap;
                ui.vertical(content);
            })
            .response;

        ui.ctx().data_mut(|data| {
            if let Some(previous) = previous {
                data.insert_temp(key, previous);
            } else {
                data.remove::<super::property_grid_context::PropertyGridContext>(key);
            }
        });

        response
    }
}
