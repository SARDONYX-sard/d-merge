//! Show method for Sheet — renders a slide-in panel overlay with animation.

impl super::sheet::Sheet {
    pub fn show(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        let anim_id = egui::Id::new("sheet_anim");
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
            egui::Id::new("sheet_backdrop"),
        );
        ctx.layer_painter(backdrop_layer).rect_filled(
            screen,
            egui::CornerRadius::ZERO,
            egui::Color32::from_black_alpha(backdrop_alpha),
        );

        // Backdrop click to close
        let backdrop_response = egui::Area::new(egui::Id::new("sheet_backdrop_sense"))
            .order(egui::Order::Middle)
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                let (_, response) =
                    ui.allocate_exact_size(screen.size(), egui::Sense::click());
                response
            });

        if backdrop_response.inner.clicked() {
            *open = false;
            ctx.request_repaint();
        }

        // Compute animated slide offset
        let slide_dist = self.width + 48.0; // panel + margin to fully hide
        let (anchor, offset) = match self.side {
            crate::tokens::sheet_side::SheetSide::Right => (
                egui::Align2::RIGHT_TOP,
                egui::vec2((1.0 - ease_t) * slide_dist, 0.0),
            ),
            crate::tokens::sheet_side::SheetSide::Left => (
                egui::Align2::LEFT_TOP,
                egui::vec2(-(1.0 - ease_t) * slide_dist, 0.0),
            ),
            crate::tokens::sheet_side::SheetSide::Top => (
                egui::Align2::LEFT_TOP,
                egui::vec2(0.0, -(1.0 - ease_t) * slide_dist),
            ),
            crate::tokens::sheet_side::SheetSide::Bottom => (
                egui::Align2::LEFT_BOTTOM,
                egui::vec2(0.0, (1.0 - ease_t) * slide_dist),
            ),
        };

        egui::Area::new(egui::Id::new("sheet_panel"))
            .order(egui::Order::Foreground)
            .anchor(anchor, offset)
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(theme.background)
                    .inner_margin(egui::Margin::same(24))
                    .stroke(egui::Stroke::new(1.0, theme.border));

                frame.show(ui, |ui| {
                    let is_horizontal = matches!(
                        self.side,
                        crate::tokens::sheet_side::SheetSide::Left
                            | crate::tokens::sheet_side::SheetSide::Right
                    );

                    if is_horizontal {
                        ui.set_min_size(egui::vec2(self.width, screen.height() - 48.0));
                        ui.set_max_width(self.width);
                    } else {
                        ui.set_min_width(screen.width() - 48.0);
                        ui.set_max_height(self.width);
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

                    // Title & description
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
