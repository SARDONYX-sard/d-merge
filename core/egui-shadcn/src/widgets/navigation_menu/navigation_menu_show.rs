//! Show method for NavigationMenu — renders a horizontal nav bar.

impl super::navigation_menu::NavigationMenu {
    /// Shows the navigation menu. `active` is the currently selected item index.
    /// Returns the index of the clicked item, if any.
    pub fn show(self, ui: &mut egui::Ui, active: &mut usize) -> Option<usize> {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let mut clicked = None;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (idx, label) in self.items.iter().enumerate() {
                let is_active = idx == *active;
                let font_size: f32 = 14.0;
                let h_pad: f32 = 12.0;
                let height: f32 = 36.0;

                let fg = if is_active {
                    theme.foreground
                } else {
                    theme.muted_foreground
                };

                let galley = ui.painter().layout_no_wrap(
                    label.clone(),
                    egui::FontId::proportional(font_size),
                    fg,
                );

                let desired = egui::vec2(galley.size().x + h_pad * 2.0, height);
                let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

                if ui.is_rect_visible(rect) {
                    let cr = egui::CornerRadius::same(theme.radius.round() as u8);

                    if is_active {
                        ui.painter().rect_filled(rect, cr, theme.accent);
                    } else if response.is_pointer_button_down_on() {
                        ui.painter().rect_filled(
                            rect,
                            cr,
                            crate::paint::interpolate_color::interpolate_color(
                                theme.accent,
                                theme.primary,
                                0.12,
                            ),
                        );
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, cr, theme.accent);
                    }

                    let text_pos = egui::pos2(
                        rect.center().x - galley.size().x / 2.0,
                        rect.center().y - galley.size().y / 2.0,
                    );
                    ui.painter().galley(text_pos, galley, fg);
                }

                if response.clicked() {
                    *active = idx;
                    clicked = Some(idx);
                    ui.ctx().request_repaint();
                }

                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        });

        clicked
    }
}
