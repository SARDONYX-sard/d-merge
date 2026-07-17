//! Show method for Combobox — renders a searchable dropdown.

impl super::combobox::Combobox {
    /// Shows the combobox. `selected` is the index of the selected item (or None).
    /// `search_text` holds the filter text state.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        selected: &mut Option<usize>,
        search_text: &mut String,
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let display = selected
            .and_then(|idx| self.items.get(idx))
            .cloned()
            .unwrap_or_else(|| self.placeholder.clone());

        let text_color = if selected.is_some() {
            theme.foreground
        } else {
            theme.muted_foreground
        };

        // Trigger button: framed text + chevron icon
        let icon_size: f32 = 12.0;
        let h_padding: f32 = 10.0;
        let height: f32 = 36.0;
        let width = self.width.unwrap_or(220.0).min(ui.available_width());
        let galley =
            ui.painter()
                .layout_no_wrap(display, egui::FontId::proportional(14.0), text_color);
        let desired = egui::vec2(width, height);
        let (trigger_rect, trigger) = ui.allocate_exact_size(desired, egui::Sense::click());

        if ui.is_rect_visible(trigger_rect) {
            let cr = egui::CornerRadius::same(theme.radius.round() as u8);
            let bg = if trigger.is_pointer_button_down_on() {
                crate::paint::interpolate_color::interpolate_color(
                    theme.background,
                    theme.accent,
                    0.85,
                )
            } else if trigger.hovered() {
                theme.accent
            } else {
                theme.background
            };
            ui.painter().rect_filled(trigger_rect, cr, bg);
            ui.painter().rect_stroke(
                trigger_rect,
                cr,
                egui::Stroke::new(1.0, theme.input),
                egui::epaint::StrokeKind::Inside,
            );

            let text_pos = egui::pos2(
                trigger_rect.min.x + h_padding,
                trigger_rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, text_color);

            let icon_rect = egui::Rect::from_min_size(
                egui::pos2(
                    trigger_rect.max.x - h_padding - icon_size,
                    trigger_rect.center().y - icon_size / 2.0,
                ),
                egui::vec2(icon_size, icon_size),
            );
            crate::icons::paint_icon::paint_icon(
                ui.painter(),
                icon_rect,
                &crate::icons::lucide_icon::LucideIcon::ChevronDown,
                theme.muted_foreground,
            );
        }

        if trigger.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        let popup_id = trigger.id.with("combobox_popup");

        let toggle_cmd = if trigger.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else {
            None
        };

        let cr = (theme.radius + 2.0).round() as u8;
        let themed_frame = egui::Frame::NONE
            .fill(theme.popover)
            .inner_margin(egui::Margin::same(8))
            .corner_radius(egui::CornerRadius::same(cr))
            .stroke(egui::Stroke::new(1.0, theme.border))
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(8),
            });

        let popup = egui::Popup::new(popup_id, ui.ctx().clone(), &trigger, ui.layer_id())
            .open_memory(toggle_cmd)
            .frame(themed_frame);

        let mut close = false;

        popup.show(|ui: &mut egui::Ui| {
            let popup_width = trigger_rect.width().max(200.0);
            ui.set_min_width(popup_width);
            ui.set_max_width(popup_width);

            // Themed search input
            let input_resp = crate::widgets::input::input::Input::new(search_text)
                .placeholder("Search...")
                .desired_width(ui.available_width())
                .show(ui);
            if trigger.clicked() {
                input_resp.request_focus();
            }

            ui.add_space(4.0);

            // Filtered items
            let query = search_text.to_lowercase();
            let filtered: Vec<(usize, &String)> = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| query.is_empty() || item.to_lowercase().contains(&query))
                .collect();

            if filtered.is_empty() {
                ui.label(
                    egui::RichText::new("No results found")
                        .color(theme.muted_foreground)
                        .size(13.0),
                );
            } else {
                let check_icon_size: f32 = 12.0;
                let item_left_pad: f32 = check_icon_size + 6.0;

                for (idx, label) in filtered {
                    let is_selected = *selected == Some(idx);
                    let galley = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::proportional(14.0),
                        theme.popover_foreground,
                    );
                    let desired = egui::vec2(
                        ui.available_width()
                            .max(galley.size().x + item_left_pad + 8.0),
                        galley.size().y + 8.0,
                    );
                    let (rect, r) = ui.allocate_exact_size(desired, egui::Sense::click());

                    if r.hovered() || is_selected {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(4), theme.accent);
                    }

                    if r.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    if ui.is_rect_visible(rect) {
                        // Check icon for selected item
                        if is_selected {
                            let check_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    rect.min.x + 4.0,
                                    rect.center().y - check_icon_size / 2.0,
                                ),
                                egui::vec2(check_icon_size, check_icon_size),
                            );
                            crate::icons::paint_icon::paint_icon(
                                ui.painter(),
                                check_rect,
                                &crate::icons::lucide_icon::LucideIcon::Check,
                                theme.popover_foreground,
                            );
                        }

                        ui.painter().galley(
                            egui::pos2(
                                rect.min.x + item_left_pad,
                                rect.center().y - galley.size().y / 2.0,
                            ),
                            galley,
                            theme.popover_foreground,
                        );
                    }

                    if r.clicked() {
                        *selected = Some(idx);
                        search_text.clear();
                        close = true;
                    }
                }
            }
        });

        if close {
            egui::Popup::close_id(ui.ctx(), popup_id);
            ui.ctx().request_repaint();
        }

        trigger
    }
}
