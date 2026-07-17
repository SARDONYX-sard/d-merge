//! Show method for Pagination — renders page navigation controls.

impl super::pagination::Pagination {
    /// Shows the pagination. `current` is the active page (0-indexed).
    pub fn show(self, ui: &mut egui::Ui, current: &mut usize) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let last_page = self.total_pages.saturating_sub(1);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            // Previous button
            let prev_enabled = *current > 0;
            let prev = self.icon_button(
                ui,
                &theme,
                &crate::icons::lucide_icon::LucideIcon::ChevronLeft,
                prev_enabled,
            );
            if prev.clicked() && prev_enabled {
                *current = current.saturating_sub(1);
                ui.ctx().request_repaint();
            }

            // Page numbers
            let half = self.max_visible / 2;
            let start = if *current <= half {
                0
            } else if *current + half >= last_page {
                last_page.saturating_sub(self.max_visible - 1)
            } else {
                *current - half
            };
            let end = (start + self.max_visible).min(self.total_pages);

            if start > 0 {
                let btn = self.page_button(ui, &theme, "1", false, true);
                if btn.clicked() {
                    *current = 0;
                    ui.ctx().request_repaint();
                }
                if start > 1 {
                    self.ellipsis_indicator(ui, &theme);
                }
            }

            for page in start..end {
                let label = format!("{}", page + 1);
                let is_current = page == *current;
                let btn = self.page_button(ui, &theme, &label, is_current, true);
                if btn.clicked() && !is_current {
                    *current = page;
                    ui.ctx().request_repaint();
                }
            }

            if end < self.total_pages {
                if end < self.total_pages - 1 {
                    self.ellipsis_indicator(ui, &theme);
                }
                let label = format!("{}", self.total_pages);
                let btn = self.page_button(ui, &theme, &label, false, true);
                if btn.clicked() {
                    *current = last_page;
                    ui.ctx().request_repaint();
                }
            }

            // Next button
            let next_enabled = *current < last_page;
            let next = self.icon_button(
                ui,
                &theme,
                &crate::icons::lucide_icon::LucideIcon::ChevronRight,
                next_enabled,
            );
            if next.clicked() && next_enabled {
                *current = (*current + 1).min(last_page);
                ui.ctx().request_repaint();
            }
        })
        .response
    }

    fn icon_button(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::theme::shadcn_theme::ShadcnTheme,
        icon: &crate::icons::lucide_icon::LucideIcon,
        enabled: bool,
    ) -> egui::Response {
        let size: f32 = 32.0;
        let icon_size: f32 = 14.0;

        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = egui::CornerRadius::same(theme.radius.round() as u8);

            let bg = if response.is_pointer_button_down_on() && enabled {
                crate::paint::interpolate_color::interpolate_color(
                    theme.accent,
                    theme.primary,
                    0.12,
                )
            } else if response.hovered() && enabled {
                theme.accent
            } else {
                egui::Color32::TRANSPARENT
            };

            painter.rect_filled(rect, cr, bg);
            painter.rect_stroke(
                rect,
                cr,
                egui::Stroke::new(1.0, theme.border),
                egui::epaint::StrokeKind::Inside,
            );

            let fg = if enabled {
                theme.foreground
            } else {
                theme.muted_foreground
            };

            let icon_rect =
                egui::Rect::from_center_size(rect.center(), egui::vec2(icon_size, icon_size));
            crate::icons::paint_icon::paint_icon(painter, icon_rect, icon, fg);
        }

        if response.hovered() && enabled {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }

    fn ellipsis_indicator(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::theme::shadcn_theme::ShadcnTheme,
    ) {
        let size: f32 = 32.0;
        let icon_size: f32 = 14.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            let icon_rect =
                egui::Rect::from_center_size(rect.center(), egui::vec2(icon_size, icon_size));
            crate::icons::paint_icon::paint_icon(
                ui.painter(),
                icon_rect,
                &crate::icons::lucide_icon::LucideIcon::Ellipsis,
                theme.muted_foreground,
            );
        }
    }

    fn page_button(
        &self,
        ui: &mut egui::Ui,
        theme: &crate::theme::shadcn_theme::ShadcnTheme,
        label: &str,
        is_active: bool,
        enabled: bool,
    ) -> egui::Response {
        let size: f32 = 32.0;
        let font_size: f32 = 14.0;

        let fg = if !enabled {
            theme.muted_foreground
        } else if is_active {
            theme.primary_foreground
        } else {
            theme.foreground
        };

        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(font_size),
            fg,
        );

        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = egui::CornerRadius::same(theme.radius.round() as u8);

            let bg = if is_active {
                theme.primary
            } else if response.is_pointer_button_down_on() && enabled {
                crate::paint::interpolate_color::interpolate_color(
                    theme.accent,
                    theme.primary,
                    0.12,
                )
            } else if response.hovered() && enabled {
                theme.accent
            } else {
                egui::Color32::TRANSPARENT
            };

            painter.rect_filled(rect, cr, bg);

            if !is_active {
                painter.rect_stroke(
                    rect,
                    cr,
                    egui::Stroke::new(1.0, theme.border),
                    egui::epaint::StrokeKind::Inside,
                );
            }

            let text_pos = egui::pos2(
                rect.center().x - galley.size().x / 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, fg);
        }

        if response.hovered() && enabled && !is_active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }
}
