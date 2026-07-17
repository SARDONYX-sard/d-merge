//! Show method for Breadcrumb — renders a navigation path.

impl super::breadcrumb::Breadcrumb {
    /// Shows the breadcrumb. Returns the index of the clicked item, if any.
    pub fn show(self, ui: &mut egui::Ui) -> Option<usize> {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let mut clicked_idx = None;
        let last_idx = self.items.len().saturating_sub(1);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            for (idx, item) in self.items.iter().enumerate() {
                let is_last = idx == last_idx;
                let font_id = egui::FontId::proportional(14.0);
                let base_color = if is_last {
                    theme.foreground
                } else {
                    theme.muted_foreground
                };
                let galley = ui
                    .painter()
                    .layout_no_wrap(item.clone(), font_id.clone(), base_color);
                let padding = egui::vec2(6.0, 3.0);
                let desired = galley.size() + padding * 2.0;
                let (rect, response) = ui.allocate_exact_size(
                    desired,
                    if is_last {
                        egui::Sense::hover()
                    } else {
                        egui::Sense::click()
                    },
                );

                if !is_last {
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    if response.clicked() {
                        clicked_idx = Some(idx);
                    }
                }

                if ui.is_rect_visible(rect) {
                    let hovered = response.hovered() && !is_last;
                    let pressed = response.is_pointer_button_down_on() && !is_last;
                    let color = if pressed {
                        theme.foreground
                    } else if hovered {
                        theme.accent_foreground
                    } else {
                        base_color
                    };

                    if hovered || pressed {
                        let bg = if pressed {
                            crate::paint::interpolate_color::interpolate_color(
                                theme.accent,
                                theme.primary,
                                0.12,
                            )
                        } else {
                            theme.accent
                        };
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::same((theme.radius * 0.75).round() as u8),
                            bg,
                        );
                    }

                    let galley_size = galley.size();
                    let text_pos = egui::pos2(
                        rect.min.x + padding.x,
                        rect.center().y - galley_size.y / 2.0,
                    );
                    ui.painter().galley(text_pos, galley, color);

                    if hovered {
                        let underline_y = text_pos.y + galley_size.y;
                        ui.painter().hline(
                            text_pos.x..=text_pos.x + rect.width() - padding.x * 2.0,
                            underline_y,
                            egui::Stroke::new(1.0, color),
                        );
                    }
                }

                if !is_last {
                    ui.label(
                        egui::RichText::new(&self.separator)
                            .color(theme.muted_foreground)
                            .size(14.0),
                    );
                }
            }
        });

        clicked_idx
    }
}
