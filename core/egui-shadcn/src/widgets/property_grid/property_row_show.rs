//! Show method for PropertyRow.

impl super::property_row::PropertyRow {
    /// Renders a property label and a control area.
    pub fn show(self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let key = super::property_grid_context_key::property_grid_context_key();
        let context = ui
            .ctx()
            .data(|data| data.get_temp::<super::property_grid_context::PropertyGridContext>(key));
        let label_width = self
            .label_width
            .or_else(|| context.map(|ctx| ctx.label_width))
            .unwrap_or(112.0);

        ui.horizontal(|ui| {
            let label_rect = ui
                .allocate_exact_size(
                    egui::vec2(label_width, ui.spacing().interact_size.y),
                    egui::Sense::hover(),
                )
                .0;
            let galley = ui.painter().layout_no_wrap(
                self.label,
                egui::FontId::proportional(12.0),
                theme.muted_foreground,
            );
            ui.painter().galley(
                egui::pos2(
                    label_rect.min.x,
                    label_rect.center().y - galley.size().y / 2.0,
                ),
                galley,
                theme.muted_foreground,
            );
            content(ui);
        })
        .response
    }
}
