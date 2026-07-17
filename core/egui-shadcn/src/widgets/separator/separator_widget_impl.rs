//! Widget trait implementation for Separator.

impl egui::Widget for super::separator::Separator {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        if self.horizontal {
            if let Some(text) = &self.text {
                // Separator with centered text label
                let galley = ui.painter().layout_no_wrap(
                    text.clone(),
                    egui::FontId::proportional(12.0),
                    theme.muted_foreground,
                );
                let text_w = galley.size().x;
                let text_h = galley.size().y;
                let available_w = ui.available_width();
                let desired = egui::vec2(available_w, text_h + 8.0);
                let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

                if ui.is_rect_visible(rect) {
                    let painter = ui.painter();
                    let cy = rect.center().y;
                    let gap = 8.0;
                    let text_x = rect.center().x - text_w / 2.0;

                    // Left line
                    if text_x - gap > rect.min.x {
                        painter.hline(
                            rect.min.x..=(text_x - gap),
                            cy,
                            egui::Stroke::new(1.0, theme.border),
                        );
                    }
                    // Right line
                    if text_x + text_w + gap < rect.max.x {
                        painter.hline(
                            (text_x + text_w + gap)..=rect.max.x,
                            cy,
                            egui::Stroke::new(1.0, theme.border),
                        );
                    }
                    // Text
                    painter.galley(
                        egui::pos2(text_x, rect.center().y - text_h / 2.0),
                        galley,
                        theme.muted_foreground,
                    );
                }

                response
            } else {
                let desired = egui::vec2(ui.available_width(), 1.0);
                let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

                if ui.is_rect_visible(rect) {
                    ui.painter().hline(
                        rect.x_range(),
                        rect.center().y,
                        egui::Stroke::new(1.0, theme.border),
                    );
                }

                response
            }
        } else {
            // Use a modest height instead of available_height() to avoid inflating
            // the row height in horizontal layouts. The line is painted over the
            // full row via min_rect after all siblings are laid out.
            let height = ui.spacing().interact_size.y.max(16.0);
            let desired = egui::vec2(1.0, height);
            let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

            if ui.is_rect_visible(rect) {
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    egui::Stroke::new(1.0, theme.border),
                );
            }

            response
        }
    }
}
