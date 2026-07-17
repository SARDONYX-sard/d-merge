//! Show method for ToggleGroup — renders a set of exclusive toggles.

impl super::toggle_group::ToggleGroup {
    /// Shows the toggle group. `selected` is the index of the active item.
    /// Returns the new selected index if changed.
    pub fn show(self, ui: &mut egui::Ui, selected: &mut usize) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let cr = theme.radius.round() as u8;

        let outer_frame = egui::Frame::NONE
            .fill(theme.muted)
            .inner_margin(egui::Margin::same(2))
            .corner_radius(egui::CornerRadius::same(cr));

        outer_frame
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (idx, label) in self.items.iter().enumerate() {
                        let is_selected = idx == *selected;
                        let response = self.render_item(ui, &theme, label, is_selected, cr);
                        if response.clicked() {
                            *selected = idx;
                            ui.ctx().request_repaint();
                        }
                    }
                });
            })
            .response
    }

    fn render_item(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::theme::shadcn_theme::ShadcnTheme,
        label: &str,
        is_selected: bool,
        cr: u8,
    ) -> egui::Response {
        let font_size: f32 = 13.0;
        let h_pad: f32 = 10.0;
        let height: f32 = 28.0;

        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(font_size),
            theme.foreground,
        );

        let desired = egui::vec2(galley.size().x + h_pad * 2.0, height);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let corner = egui::CornerRadius::same(cr.saturating_sub(1));

            let (bg, fg) = if is_selected {
                (theme.background, theme.foreground)
            } else if response.hovered() {
                (
                    egui::Color32::from_rgba_unmultiplied(
                        theme.background.r(),
                        theme.background.g(),
                        theme.background.b(),
                        128,
                    ),
                    theme.foreground,
                )
            } else {
                (egui::Color32::TRANSPARENT, theme.muted_foreground)
            };

            painter.rect_filled(rect, corner, bg);

            if is_selected {
                painter.rect_stroke(
                    rect,
                    corner,
                    egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(
                            theme.foreground.r(),
                            theme.foreground.g(),
                            theme.foreground.b(),
                            13,
                        ),
                    ),
                    egui::epaint::StrokeKind::Inside,
                );
            }

            let text_pos = egui::pos2(
                rect.center().x - galley.size().x / 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, fg);
        }

        response
    }
}
