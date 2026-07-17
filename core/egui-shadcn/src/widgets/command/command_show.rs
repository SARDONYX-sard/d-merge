//! Show method for Command — renders a command palette overlay.

impl super::command::Command {
    /// Shows the command palette when `open` is true.
    /// `search` holds the filter text. Returns the index of selected command if any.
    pub fn show(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        search: &mut String,
    ) -> Option<usize> {
        if !*open {
            return None;
        }

        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ctx);
        let mut selected = None;

        // Backdrop
        let screen = ctx.input(|i| i.viewport_rect());
        let backdrop_layer = egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("command_backdrop"),
        );
        ctx.layer_painter(backdrop_layer).rect_filled(
            screen,
            egui::CornerRadius::ZERO,
            egui::Color32::from_black_alpha(60),
        );

        // Backdrop click to close
        let backdrop_resp = egui::Area::new(egui::Id::new("command_backdrop_sense"))
            .order(egui::Order::Middle)
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                let (_, response) =
                    ui.allocate_exact_size(screen.size(), egui::Sense::click());
                response
            });

        if backdrop_resp.inner.clicked() {
            *open = false;
            search.clear();
            ctx.request_repaint();
            return None;
        }

        // Escape to close
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            search.clear();
            ctx.request_repaint();
            return None;
        }

        let cr = (theme.radius + 2.0).round() as u8;

        egui::Area::new(egui::Id::new("command_palette"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -60.0))
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(theme.popover)
                    .inner_margin(egui::Margin::same(0))
                    .corner_radius(egui::CornerRadius::same(cr))
                    .stroke(egui::Stroke::new(1.0, theme.border))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(12),
                    });

                frame.show(ui, |ui| {
                    ui.set_min_width(480.0);
                    ui.set_max_width(480.0);

                    // Search input
                    let input_frame = egui::Frame::NONE.inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 12,
                        bottom: 12,
                    });

                    input_frame.show(ui, |ui| {
                        let input_resp =
                            crate::widgets::input::input::Input::new(search)
                                .placeholder(&self.placeholder)
                                .desired_width(ui.available_width())
                                .show(ui);
                        input_resp.request_focus();
                    });

                    // Divider
                    let avail = ui.available_rect_before_wrap();
                    ui.painter().hline(
                        avail.min.x..=avail.max.x,
                        avail.min.y,
                        egui::Stroke::new(1.0, theme.border),
                    );
                    ui.add_space(1.0);

                    // Command list
                    let query = search.to_lowercase();
                    let results_frame =
                        egui::Frame::NONE.inner_margin(egui::Margin::same(8));

                    results_frame.show(ui, |ui| {
                        let mut current_group = String::new();
                        let mut any_shown = false;

                        for (idx, (group, label)) in self.items.iter().enumerate() {
                            if !query.is_empty()
                                && !label.to_lowercase().contains(&query)
                                && !group.to_lowercase().contains(&query)
                            {
                                continue;
                            }

                            any_shown = true;

                            if *group != current_group {
                                if !current_group.is_empty() {
                                    ui.add_space(4.0);
                                }
                                ui.label(
                                    egui::RichText::new(group)
                                        .color(theme.muted_foreground)
                                        .size(12.0)
                                        .strong(),
                                );
                                ui.add_space(2.0);
                                current_group = group.clone();
                            }

                            let galley = ui.painter().layout_no_wrap(
                                label.clone(),
                                egui::FontId::proportional(14.0),
                                theme.popover_foreground,
                            );
                            let desired = egui::vec2(
                                ui.available_width(),
                                galley.size().y + 8.0,
                            );
                            let (rect, r) =
                                ui.allocate_exact_size(desired, egui::Sense::click());

                            if r.hovered() {
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
                                    theme.popover_foreground,
                                );
                            }

                            if r.clicked() {
                                selected = Some(idx);
                                *open = false;
                                search.clear();
                                ctx.request_repaint();
                            }
                        }

                        if !any_shown {
                            ui.label(
                                egui::RichText::new("No results found.")
                                    .color(theme.muted_foreground)
                                    .size(14.0),
                            );
                        }
                    });
                });
            });

        selected
    }
}
