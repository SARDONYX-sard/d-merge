//! Show method for Dialog — renders a modal overlay.

impl super::dialog::Dialog {
    /// Shows the dialog when `open` is true. Content closure receives a `&mut Ui`.
    pub fn show(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        if !*open {
            return;
        }

        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ctx);

        // Backdrop
        let screen = ctx.input(|i| i.viewport_rect());
        let backdrop_layer = egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("dialog_backdrop"),
        );
        let painter = ctx.layer_painter(backdrop_layer);
        painter.rect_filled(
            screen,
            egui::CornerRadius::ZERO,
            egui::Color32::from_black_alpha(60),
        );

        // Consume clicks on backdrop to close
        let backdrop_response = egui::Area::new(egui::Id::new("dialog_backdrop_sense"))
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
            return;
        }

        // Dialog panel
        let cr = (theme.radius + 2.0).round() as u8;

        egui::Area::new(egui::Id::new("dialog_panel"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(theme.background)
                    .inner_margin(egui::Margin::same(24))
                    .corner_radius(egui::CornerRadius::same(cr))
                    .stroke(egui::Stroke::new(1.0, theme.border))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(12),
                    });

                frame.show(ui, |ui| {
                    ui.set_max_width(self.width);

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
    }
}
