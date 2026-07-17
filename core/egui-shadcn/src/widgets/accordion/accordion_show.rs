//! Show method for Accordion -- renders collapsible sections with dividers.

impl super::accordion::Accordion {
    /// Shows the accordion. `open_indices` tracks which sections are expanded.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        open_indices: &mut Vec<usize>,
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        ui.vertical(|ui| {
            for (idx, (title, content)) in self.items.iter().enumerate() {
                let is_open = open_indices.contains(&idx);

                // Divider line (top border for each section)
                if idx > 0 {
                    let rect = ui.available_rect_before_wrap();
                    let line_y = rect.min.y;
                    ui.painter().hline(
                        rect.min.x..=rect.max.x,
                        line_y,
                        egui::Stroke::new(1.0, theme.border),
                    );
                }

                // Section header
                ui.add_space(12.0);
                let header_response = ui.horizontal(|ui| {
                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let trigger = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(title)
                                        .color(theme.foreground)
                                        .size(14.0)
                                        .strong(),
                                )
                                .sense(egui::Sense::click()),
                            );

                            // Expand to fill width then add chevron on right
                            let icon_size: f32 = 14.0;
                            let remaining = ui.available_width() - icon_size - 4.0;
                            if remaining > 0.0 {
                                ui.add_space(remaining);
                            }

                            let chevron_icon = if is_open {
                                crate::icons::lucide_icon::LucideIcon::Minus
                            } else {
                                crate::icons::lucide_icon::LucideIcon::Plus
                            };
                            let (icon_rect, _) = ui.allocate_exact_size(
                                egui::vec2(icon_size, icon_size),
                                egui::Sense::hover(),
                            );
                            if ui.is_rect_visible(icon_rect) {
                                crate::icons::paint_icon::paint_icon(
                                    ui.painter(),
                                    icon_rect,
                                    &chevron_icon,
                                    theme.muted_foreground,
                                );
                            }

                            trigger
                        },
                    )
                    .inner
                });

                let trigger = header_response.inner;

                if trigger.clicked() {
                    if is_open {
                        open_indices.retain(|&i| i != idx);
                    } else {
                        if !self.multiple {
                            open_indices.clear();
                        }
                        open_indices.push(idx);
                    }
                    ui.ctx().request_repaint();
                }

                if trigger.hovered() {
                    ui.ctx()
                        .set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                // Content
                if is_open {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(content)
                            .color(theme.muted_foreground)
                            .size(14.0),
                    );
                }

                ui.add_space(12.0);
            }
        })
        .response
    }
}
