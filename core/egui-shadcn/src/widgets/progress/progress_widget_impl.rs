//! Widget trait implementation for Progress.

impl egui::Widget for super::progress::Progress {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let height: f32 = 8.0;
        let desired = egui::vec2(ui.available_width(), height);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = egui::CornerRadius::same(255); // rounded-full

            // Track background (primary at 20% opacity)
            let track_color = egui::Color32::from_rgba_unmultiplied(
                theme.primary.r(),
                theme.primary.g(),
                theme.primary.b(),
                51, // ~20%
            );
            painter.rect_filled(rect, cr, track_color);

            // Fill bar
            if self.value > 0.0 {
                let fill_width = rect.width() * self.value;
                let fill_rect = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(fill_width, height),
                );
                painter.rect_filled(fill_rect, cr, theme.primary);
            }
        }

        response
    }
}
