//! Show methods for DropdownMenu — renders a popup menu.

impl super::dropdown_menu::DropdownMenu {
    /// Shows a dropdown menu with simple string items (backward-compatible API).
    pub fn show(
        ui: &mut egui::Ui,
        trigger_response: &egui::Response,
        items: &[&str],
        on_select: impl FnOnce(usize),
    ) {
        let menu_items: Vec<super::dropdown_menu::MenuItem> = items
            .iter()
            .map(|label| super::dropdown_menu::MenuItem::label(*label))
            .collect();
        Self::show_rich(ui, trigger_response, &menu_items, on_select);
    }

    /// Shows a dropdown menu with rich `MenuItem`s (shortcuts, separators, disabled items).
    pub fn show_rich(
        ui: &mut egui::Ui,
        trigger_response: &egui::Response,
        items: &[super::dropdown_menu::MenuItem],
        on_select: impl FnOnce(usize),
    ) {
        let popup_id = trigger_response.id.with("dropdown");
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let toggle_cmd = if trigger_response.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else {
            None
        };

        let cr = (theme.radius + 2.0).round() as u8;
        let themed_frame = egui::Frame::NONE
            .fill(theme.popover)
            .inner_margin(egui::Margin::same(4))
            .corner_radius(egui::CornerRadius::same(cr))
            .stroke(egui::Stroke::new(1.0, theme.border))
            .shadow(egui::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(8),
            });

        let popup = egui::Popup::new(
            popup_id,
            ui.ctx().clone(),
            trigger_response,
            ui.layer_id(),
        )
        .open_memory(toggle_cmd)
        .frame(themed_frame);

        let mut selected_idx = None;

        popup.show(|ui: &mut egui::Ui| {
            // Compute max text width
            let mut max_label_w: f32 = 0.0;
            let mut max_shortcut_w: f32 = 0.0;

            for item in items {
                if let super::dropdown_menu::MenuItem::Item { label, shortcut, .. } = item {
                    let lw = ui.painter()
                        .layout_no_wrap(
                            label.clone(),
                            egui::FontId::proportional(14.0),
                            theme.popover_foreground,
                        )
                        .size()
                        .x;
                    max_label_w = max_label_w.max(lw);

                    if let Some(sc) = shortcut {
                        let sw = ui.painter()
                            .layout_no_wrap(
                                sc.clone(),
                                egui::FontId::proportional(12.0),
                                theme.muted_foreground,
                            )
                            .size()
                            .x;
                        max_shortcut_w = max_shortcut_w.max(sw);
                    }
                }
            }

            let shortcut_space = if max_shortcut_w > 0.0 { max_shortcut_w + 24.0 } else { 0.0 };
            let menu_width = (max_label_w + shortcut_space + 24.0).max(120.0);
            ui.set_min_width(menu_width);
            ui.set_max_width(menu_width);

            let mut item_idx = 0;
            for item in items {
                match item {
                    super::dropdown_menu::MenuItem::Item { label, shortcut, enabled } => {
                        let current_idx = item_idx;
                        item_idx += 1;

                        let fg = if *enabled {
                            theme.popover_foreground
                        } else {
                            theme.muted_foreground
                        };

                        let galley = ui.painter().layout_no_wrap(
                            label.clone(),
                            egui::FontId::proportional(14.0),
                            fg,
                        );
                        let desired = egui::vec2(menu_width, galley.size().y + 8.0);
                        let (rect, r) = ui.allocate_exact_size(desired, egui::Sense::click());

                        if *enabled && r.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(4),
                                theme.accent,
                            );
                        }

                        if ui.is_rect_visible(rect) {
                            ui.painter().galley(
                                egui::pos2(
                                    rect.min.x + 8.0,
                                    rect.center().y - galley.size().y / 2.0,
                                ),
                                galley,
                                fg,
                            );

                            if let Some(sc) = shortcut {
                                let sc_galley = ui.painter().layout_no_wrap(
                                    sc.clone(),
                                    egui::FontId::proportional(12.0),
                                    theme.muted_foreground,
                                );
                                ui.painter().galley(
                                    egui::pos2(
                                        rect.max.x - 8.0 - sc_galley.size().x,
                                        rect.center().y - sc_galley.size().y / 2.0,
                                    ),
                                    sc_galley,
                                    theme.muted_foreground,
                                );
                            }
                        }

                        if *enabled && r.clicked() {
                            selected_idx = Some(current_idx);
                            egui::Popup::close_id(ui.ctx(), popup_id);
                            ui.ctx().request_repaint();
                        }
                    }
                    super::dropdown_menu::MenuItem::Separator => {
                        ui.add_space(2.0);
                        let (sep_rect, _) = ui.allocate_exact_size(
                            egui::vec2(menu_width, 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            sep_rect,
                            egui::CornerRadius::ZERO,
                            theme.border,
                        );
                        ui.add_space(2.0);
                    }
                }
            }
        });

        if let Some(idx) = selected_idx {
            on_select(idx);
        }
    }
}
