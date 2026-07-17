//! Widget trait implementation for Switch.

impl egui::Widget for super::switch::Switch<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let track_w: f32 = 32.0;
        let track_h: f32 = 18.4;
        let thumb_size: f32 = 14.4;
        let thumb_margin: f32 = 2.0;
        let spacing: f32 = 8.0;

        let label_galley = self.label.map(|l| {
            ui.painter().layout_no_wrap(
                l.text().to_owned(),
                egui::FontId::proportional(14.0),
                theme.foreground,
            )
        });

        let label_width = label_galley.as_ref().map_or(0.0, |g| g.size().x + spacing);
        let desired = egui::vec2(track_w + label_width, track_h.max(20.0));

        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

        if response.clicked() {
            *self.on = !*self.on;
            response.ctx.request_repaint();
        }

        let anim_t = ui.ctx().animate_bool_responsive(response.id, *self.on);

        if ui.is_rect_visible(rect) {
            let style = super::switch_style::resolve_switch_style(&theme, *self.on, anim_t);
            let painter = ui.painter();

            // Track
            let track_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, rect.center().y - track_h / 2.0),
                egui::vec2(track_w, track_h),
            );
            let track_cr = (track_h / 2.0).round().min(255.0) as u8;
            let track_color = if response.is_pointer_button_down_on() {
                crate::paint::interpolate_color::interpolate_color(
                    style.track_color,
                    theme.accent,
                    0.45,
                )
            } else if response.hovered() {
                crate::paint::interpolate_color::interpolate_color(
                    style.track_color,
                    theme.accent,
                    0.25,
                )
            } else {
                style.track_color
            };
            painter.rect_filled(track_rect, egui::CornerRadius::same(track_cr), track_color);
            let track_border = if response.hovered() || response.is_pointer_button_down_on() {
                Some(theme.ring)
            } else {
                style.track_border
            };
            if let Some(border_color) = track_border {
                painter.rect_stroke(
                    track_rect,
                    egui::CornerRadius::same(track_cr),
                    egui::Stroke::new(1.0, border_color),
                    egui::epaint::StrokeKind::Inside,
                );
            }

            // Thumb - slides from left to right
            let thumb_min_x = track_rect.min.x + thumb_margin;
            let thumb_max_x = track_rect.max.x - thumb_margin - thumb_size;
            let thumb_x = thumb_min_x + (thumb_max_x - thumb_min_x) * anim_t;
            let thumb_center = egui::pos2(thumb_x + thumb_size / 2.0, track_rect.center().y);
            painter.circle_filled(thumb_center, thumb_size / 2.0, style.thumb_color);

            // Label
            if let Some(galley) = label_galley {
                let text_pos = egui::pos2(
                    track_rect.max.x + spacing,
                    rect.center().y - galley.size().y / 2.0,
                );
                painter.galley(text_pos, galley, theme.foreground);
            }

            // Focus ring
            if response.has_focus() {
                crate::paint::paint_focus_ring::paint_focus_ring(
                    painter,
                    track_rect,
                    track_h / 2.0,
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
