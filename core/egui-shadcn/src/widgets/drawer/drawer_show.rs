//! Show method for Drawer — renders a bottom slide-up panel with animation.

impl super::drawer::Drawer {
    pub fn show(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        let anim_id = egui::Id::new("drawer_anim");
        let anim_t = ctx.animate_bool_with_time(anim_id, *open, 0.2);

        if anim_t <= 0.0 {
            return;
        }

        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ctx);
        let screen = ctx.input(|i| i.viewport_rect());
        let ease_t = ease_out_cubic(anim_t);

        // Animated backdrop
        let backdrop_alpha = (60.0 * ease_t) as u8;
        let backdrop_layer = egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("drawer_backdrop"),
        );
        ctx.layer_painter(backdrop_layer).rect_filled(
            screen,
            egui::CornerRadius::ZERO,
            egui::Color32::from_black_alpha(backdrop_alpha),
        );

        // Backdrop click to close
        let backdrop_resp = egui::Area::new(egui::Id::new("drawer_backdrop_sense"))
            .order(egui::Order::Middle)
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                let (_, response) =
                    ui.allocate_exact_size(screen.size(), egui::Sense::click());
                response
            });

        if backdrop_resp.inner.clicked() {
            *open = false;
            ctx.request_repaint();
        }

        let cr_top = theme.radius.round() as u8 + 2;
        let slide_offset_y = (1.0 - ease_t) * 400.0;

        egui::Area::new(egui::Id::new("drawer_panel"))
            .order(egui::Order::Foreground)
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(0.0, slide_offset_y),
            )
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(theme.background)
                    .inner_margin(egui::Margin {
                        left: 24,
                        right: 24,
                        top: 16,
                        bottom: 24,
                    })
                    .corner_radius(egui::CornerRadius {
                        nw: cr_top,
                        ne: cr_top,
                        sw: 0,
                        se: 0,
                    })
                    .stroke(egui::Stroke::new(1.0, theme.border));

                frame.show(ui, |ui| {
                    ui.set_min_width(screen.width().min(500.0));

                    // Handle bar
                    let handle_width: f32 = 48.0;
                    let handle_height: f32 = 4.0;
                    let (handle_rect, handle_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), handle_height + 8.0),
                        egui::Sense::drag(),
                    );
                    let bar_rect = egui::Rect::from_center_size(
                        handle_rect.center(),
                        egui::vec2(handle_width, handle_height),
                    );
                    ui.painter().rect_filled(
                        bar_rect,
                        egui::CornerRadius::same(255),
                        theme.muted_foreground,
                    );

                    // Track cumulative drag; close when dragged down past threshold
                    let drag_id = egui::Id::new("drawer_drag_delta");
                    let mut cumulative: f32 =
                        ctx.data(|d| d.get_temp(drag_id).unwrap_or(0.0_f32));
                    if handle_resp.dragged() {
                        cumulative += handle_resp.drag_delta().y;
                        ctx.data_mut(|d| d.insert_temp(drag_id, cumulative));
                    }
                    if handle_resp.drag_stopped() {
                        if cumulative > 80.0 {
                            *open = false;
                            ctx.request_repaint();
                        }
                        ctx.data_mut(|d| d.insert_temp(drag_id, 0.0_f32));
                    }

                    // Close button
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::TOP),
                        |ui| {
                            let close_size = 16.0;
                            let (close_rect, close_resp) = ui.allocate_exact_size(
                                egui::vec2(close_size, close_size),
                                egui::Sense::click(),
                            );
                            if ui.is_rect_visible(close_rect) {
                                crate::icons::paint_icon::paint_icon(
                                    ui.painter(),
                                    close_rect,
                                    &crate::icons::lucide_icon::LucideIcon::X,
                                    theme.muted_foreground,
                                );
                            }
                            if close_resp.clicked() {
                                *open = false;
                                ctx.request_repaint();
                            }
                        },
                    );

                    // Title and description
                    if let Some(title) = self.title {
                        ui.label(
                            egui::RichText::new(title)
                                .color(theme.foreground)
                                .size(18.0)
                                .strong(),
                        );
                    }

                    if let Some(desc) = self.description {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(desc)
                                .color(theme.muted_foreground)
                                .size(14.0),
                        );
                    }

                    ui.add_space(16.0);
                    content(ui);
                });
            });

        ctx.request_repaint();
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
