//! Widget trait implementation for Kbd.

impl egui::Widget for super::kbd::Kbd {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let fg = theme.foreground;
        let fill =
            crate::paint::interpolate_color::interpolate_color(theme.background, theme.muted, 0.75);

        let galley = ui
            .painter()
            .layout_no_wrap(self.text, egui::FontId::monospace(11.0), fg);

        let h_padding: f32 = 6.0;
        let v_padding: f32 = 2.0;
        let desired = egui::vec2(
            galley.size().x + h_padding * 2.0,
            galley.size().y + v_padding * 2.0,
        );

        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = egui::CornerRadius::same(6);

            painter.rect_filled(rect, cr, fill);
            painter.rect_stroke(
                rect,
                cr,
                egui::Stroke::new(1.0, theme.input),
                egui::epaint::StrokeKind::Inside,
            );

            let text_pos = egui::pos2(
                rect.center().x - galley.size().x / 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, fg);
        }

        response
    }
}
