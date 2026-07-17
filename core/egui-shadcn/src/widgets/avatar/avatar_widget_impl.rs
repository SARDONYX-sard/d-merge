//! Widget trait implementation for Avatar.

impl egui::Widget for super::avatar::Avatar {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let desired = egui::vec2(self.size, self.size);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            let radius = self.size / 2.0;

            // Background circle
            painter.circle_filled(center, radius, theme.muted);

            // Initials text
            let font_size = self.size * 0.4;
            let galley = painter.layout_no_wrap(
                self.initials,
                egui::FontId::proportional(font_size),
                theme.muted_foreground,
            );
            let text_pos = egui::pos2(
                center.x - galley.size().x / 2.0,
                center.y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, theme.muted_foreground);
        }

        response
    }
}
