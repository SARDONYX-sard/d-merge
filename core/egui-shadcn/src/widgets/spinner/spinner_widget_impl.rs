//! Widget trait implementation for Spinner.

impl egui::Widget for super::spinner::Spinner {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let desired = egui::vec2(self.size, self.size);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let time = ui.input(|i| i.time) as f32;
            let angle_offset = time * std::f32::consts::TAU; // 1 revolution per second

            crate::paint::paint_arc::paint_arc(
                ui.painter(),
                rect.center(),
                self.size / 2.0 - 1.0,
                angle_offset,
                egui::Stroke::new(2.0, theme.foreground),
            );
        }

        ui.ctx().request_repaint();
        response
    }
}
