//! Widget trait implementation for Select and SelectValue.

impl<T: Clone + std::fmt::Display + PartialEq + 'static> egui::Widget
    for super::select::Select<'_, T>
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let style = super::select_style::resolve_select_style(&theme);

        let height: f32 = 32.0;
        let h_padding: f32 = 10.0;
        let chevron_width: f32 = 20.0;
        let width = self.width.unwrap_or(ui.available_width().min(200.0));
        let desired = egui::vec2(width, height);

        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
        let popup_id = response.id.with("popup");

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = egui::CornerRadius::same(style.corner_radius.round() as u8);
            let pressed = response.is_pointer_button_down_on();
            let trigger_bg = if pressed {
                crate::paint::interpolate_color::interpolate_color(
                    style.trigger_bg,
                    theme.accent,
                    0.85,
                )
            } else if response.hovered() {
                theme.accent
            } else {
                style.trigger_bg
            };
            let trigger_border = if response.hovered() || pressed {
                theme.ring
            } else {
                style.trigger_border
            };

            painter.rect_filled(rect, cr, trigger_bg);
            painter.rect_stroke(
                rect,
                cr,
                egui::Stroke::new(1.0, trigger_border),
                egui::epaint::StrokeKind::Inside,
            );

            // Display text — use override if provided
            let display_text = if let Some(ref override_text) = self.selected_text_override {
                override_text.clone()
            } else {
                match &self.selected {
                    Some(val) => val.to_string(),
                    None => self.placeholder.clone(),
                }
            };

            let text_color = if self.selected.is_some() || self.selected_text_override.is_some() {
                style.trigger_text
            } else {
                theme.muted_foreground
            };

            let galley =
                painter.layout_no_wrap(display_text, egui::FontId::proportional(14.0), text_color);
            let text_pos = egui::pos2(
                rect.min.x + h_padding,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, text_color);

            let icon_size: f32 = 14.0;
            let chevron_rect = egui::Rect::from_center_size(
                egui::pos2(rect.max.x - chevron_width / 2.0 - 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size),
            );
            crate::icons::paint_icon::paint_icon(
                painter,
                chevron_rect,
                &crate::icons::lucide_icon::LucideIcon::ChevronDown,
                theme.muted_foreground,
            );

            if response.has_focus() {
                crate::paint::paint_focus_ring::paint_focus_ring(
                    painter,
                    rect,
                    style.corner_radius,
                    theme.ring,
                );
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let toggle_cmd = if response.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else {
            None
        };

        let popup_cr = style.corner_radius.round() as u8;
        let popup = egui::Popup::new(popup_id, ui.ctx().clone(), &response, ui.layer_id())
            .open_memory(toggle_cmd)
            .frame(
                egui::Frame::NONE
                    .fill(style.popover_bg)
                    .inner_margin(egui::Margin::same(4))
                    .corner_radius(egui::CornerRadius::same(popup_cr))
                    .stroke(egui::Stroke::new(1.0, style.popover_border))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(8),
                    }),
            );

        popup.show(|ui: &mut egui::Ui| {
            let popup_width = width.max(144.0);
            ui.set_min_width(popup_width);
            ui.set_max_width(popup_width);
            let check_icon_size: f32 = 12.0;

            for option in self.options {
                let is_selected = self.selected.as_ref() == Some(option);
                let label = option.to_string();

                let galley = ui.painter().layout_no_wrap(
                    label.clone(),
                    egui::FontId::proportional(14.0),
                    style.item_text,
                );

                let item_height = galley.size().y + 8.0;
                let item_desired = egui::vec2(popup_width, item_height);
                let (item_rect, item_response) =
                    ui.allocate_exact_size(item_desired, egui::Sense::click());

                if ui.is_rect_visible(item_rect) {
                    let item_cr = egui::CornerRadius::same(
                        (style.corner_radius - 2.0).max(4.0).round() as u8,
                    );
                    if item_response.hovered() {
                        ui.painter()
                            .rect_filled(item_rect, item_cr, style.item_hover_bg);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    let text_x = item_rect.min.x + 6.0;
                    if is_selected {
                        let check_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                item_rect.max.x - check_icon_size - 8.0,
                                item_rect.center().y - check_icon_size / 2.0,
                            ),
                            egui::vec2(check_icon_size, check_icon_size),
                        );
                        crate::icons::paint_icon::paint_icon(
                            ui.painter(),
                            check_rect,
                            &crate::icons::lucide_icon::LucideIcon::Check,
                            style.item_text,
                        );
                    }

                    ui.painter().galley(
                        egui::pos2(text_x, item_rect.center().y - galley.size().y / 2.0),
                        galley,
                        style.item_text,
                    );
                }

                if item_response.clicked() {
                    *self.selected = Some(option.clone());
                    egui::Popup::close_id(ui.ctx(), popup_id);
                    ui.ctx().request_repaint();
                }
            }
        });

        response
    }
}

// ---------------------------------------------------------------------------
// SelectValue — non-Option variant
// ---------------------------------------------------------------------------

impl<T: Clone + std::fmt::Display + PartialEq + 'static> egui::Widget
    for super::select::SelectValue<'_, T>
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let style = super::select_style::resolve_select_style(&theme);

        let height: f32 = 32.0;
        let h_padding: f32 = 10.0;
        let chevron_width: f32 = 20.0;
        let width = self.width.unwrap_or(ui.available_width().min(200.0));
        let desired = egui::vec2(width, height);

        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
        let popup_id = response.id.with("popup");

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let cr = egui::CornerRadius::same(style.corner_radius.round() as u8);
            let pressed = response.is_pointer_button_down_on();
            let trigger_bg = if pressed {
                crate::paint::interpolate_color::interpolate_color(
                    style.trigger_bg,
                    theme.accent,
                    0.85,
                )
            } else if response.hovered() {
                theme.accent
            } else {
                style.trigger_bg
            };
            let trigger_border = if response.hovered() || pressed {
                theme.ring
            } else {
                style.trigger_border
            };

            painter.rect_filled(rect, cr, trigger_bg);
            painter.rect_stroke(
                rect,
                cr,
                egui::Stroke::new(1.0, trigger_border),
                egui::epaint::StrokeKind::Inside,
            );

            let display_text = if let Some(ref override_text) = self.selected_text_override {
                override_text.clone()
            } else {
                self.selected.to_string()
            };

            let galley = painter.layout_no_wrap(
                display_text,
                egui::FontId::proportional(14.0),
                style.trigger_text,
            );
            let text_pos = egui::pos2(
                rect.min.x + h_padding,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, style.trigger_text);

            let icon_size: f32 = 14.0;
            let chevron_rect = egui::Rect::from_center_size(
                egui::pos2(rect.max.x - chevron_width / 2.0 - 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size),
            );
            crate::icons::paint_icon::paint_icon(
                painter,
                chevron_rect,
                &crate::icons::lucide_icon::LucideIcon::ChevronDown,
                theme.muted_foreground,
            );

            if response.has_focus() {
                crate::paint::paint_focus_ring::paint_focus_ring(
                    painter,
                    rect,
                    style.corner_radius,
                    theme.ring,
                );
            }
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let toggle_cmd = if response.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else {
            None
        };

        let popup_cr = style.corner_radius.round() as u8;
        let popup = egui::Popup::new(popup_id, ui.ctx().clone(), &response, ui.layer_id())
            .open_memory(toggle_cmd)
            .frame(
                egui::Frame::NONE
                    .fill(style.popover_bg)
                    .inner_margin(egui::Margin::same(4))
                    .corner_radius(egui::CornerRadius::same(popup_cr))
                    .stroke(egui::Stroke::new(1.0, style.popover_border))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(8),
                    }),
            );

        popup.show(|ui: &mut egui::Ui| {
            let popup_width = width.max(144.0);
            ui.set_min_width(popup_width);
            ui.set_max_width(popup_width);
            let check_icon_size: f32 = 12.0;

            for option in self.options {
                let is_selected = self.selected == option;
                let label = option.to_string();

                let galley = ui.painter().layout_no_wrap(
                    label.clone(),
                    egui::FontId::proportional(14.0),
                    style.item_text,
                );

                let item_height = galley.size().y + 8.0;
                let item_desired = egui::vec2(popup_width, item_height);
                let (item_rect, item_response) =
                    ui.allocate_exact_size(item_desired, egui::Sense::click());

                if ui.is_rect_visible(item_rect) {
                    let item_cr = egui::CornerRadius::same(
                        (style.corner_radius - 2.0).max(4.0).round() as u8,
                    );
                    if item_response.hovered() {
                        ui.painter()
                            .rect_filled(item_rect, item_cr, style.item_hover_bg);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    let text_x = item_rect.min.x + 6.0;
                    if is_selected {
                        let check_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                item_rect.max.x - check_icon_size - 8.0,
                                item_rect.center().y - check_icon_size / 2.0,
                            ),
                            egui::vec2(check_icon_size, check_icon_size),
                        );
                        crate::icons::paint_icon::paint_icon(
                            ui.painter(),
                            check_rect,
                            &crate::icons::lucide_icon::LucideIcon::Check,
                            style.item_text,
                        );
                    }

                    ui.painter().galley(
                        egui::pos2(text_x, item_rect.center().y - galley.size().y / 2.0),
                        galley,
                        style.item_text,
                    );
                }

                if item_response.clicked() {
                    *self.selected = option.clone();
                    egui::Popup::close_id(ui.ctx(), popup_id);
                    ui.ctx().request_repaint();
                }
            }
        });

        response
    }
}
