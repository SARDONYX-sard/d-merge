//! Widget trait implementation for Radio.

impl egui::Widget for super::radio::Radio<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let outer_radius = 8.0;
        let inner_radius = 4.0;
        let spacing = 8.0;

        let label_galley = self.label.map(|l| {
            ui.painter().layout_no_wrap(
                l.text().to_owned(),
                egui::FontId::proportional(14.0),
                theme.foreground,
            )
        });

        let label_width = label_galley.as_ref().map_or(0.0, |g| g.size().x + spacing);
        let desired = egui::vec2(outer_radius * 2.0 + label_width, 20.0);

        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

        if response.clicked() {
            *self.selected = true;
            response.ctx.request_repaint();
        }

        let anim_t = ui
            .ctx()
            .animate_bool_responsive(response.id, *self.selected);

        if ui.is_rect_visible(rect) {
            let mut style =
                super::radio_style::resolve_radio_style(&theme, *self.selected, response.hovered());
            if response.is_pointer_button_down_on() {
                style.circle_border = theme.ring;
            }
            let painter = ui.painter();

            let center = egui::pos2(rect.min.x + outer_radius, rect.center().y);

            // Outer circle
            painter.circle_stroke(
                center,
                outer_radius,
                egui::Stroke::new(1.0, style.circle_border),
            );

            // Inner dot (animated)
            if anim_t > 0.01 {
                painter.circle_filled(center, inner_radius * anim_t, style.dot_color);
            }

            // Label
            if let Some(galley) = label_galley {
                let text_pos = egui::pos2(
                    rect.min.x + outer_radius * 2.0 + spacing,
                    rect.center().y - galley.size().y / 2.0,
                );
                painter.galley(text_pos, galley, style.text_color);
            }

            // Focus ring
            if response.has_focus() {
                let ring_rect = egui::Rect::from_center_size(
                    center,
                    egui::vec2(outer_radius * 2.0, outer_radius * 2.0),
                );
                crate::paint::paint_focus_ring::paint_focus_ring(
                    painter,
                    ring_rect,
                    outer_radius,
                    theme.ring,
                );
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }
}
