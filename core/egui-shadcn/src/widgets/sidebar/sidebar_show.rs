//! Show method for Sidebar -- renders a fixed sidebar panel.

impl super::sidebar::Sidebar {
    /// Shows the sidebar. `collapsed` controls collapsed state if collapsible.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        collapsed: &mut bool,
        content: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let effective_width = if self.collapsible && *collapsed {
            48.0
        } else {
            self.width
        };

        let frame = egui::Frame::NONE
            .fill(theme.card)
            .inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 12,
                bottom: 12,
            })
            .stroke(egui::Stroke::new(1.0, theme.border));

        frame
            .show(ui, |ui| {
                ui.set_min_width(effective_width);
                ui.set_max_width(effective_width);
                ui.set_min_height(ui.available_height());

                ui.vertical(|ui| {
                    if self.collapsible {
                        let toggle_icon = if *collapsed {
                            crate::icons::lucide_icon::LucideIcon::PanelLeftOpen
                        } else {
                            crate::icons::lucide_icon::LucideIcon::PanelLeftClose
                        };
                        let icon_size: f32 = 16.0;
                        let (icon_rect, toggle_resp) = ui.allocate_exact_size(
                            egui::vec2(icon_size, icon_size),
                            egui::Sense::click(),
                        );
                        if ui.is_rect_visible(icon_rect) {
                            crate::icons::paint_icon::paint_icon(
                                ui.painter(),
                                icon_rect,
                                &toggle_icon,
                                theme.muted_foreground,
                            );
                        }
                        if toggle_resp.clicked() {
                            *collapsed = !*collapsed;
                            ui.ctx().request_repaint();
                        }
                        ui.add_space(8.0);
                    }

                    if !*collapsed || !self.collapsible {
                        content(ui);
                    }
                });
            })
            .response
    }
}
