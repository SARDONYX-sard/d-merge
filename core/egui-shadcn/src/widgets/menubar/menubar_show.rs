//! Show method for Menubar — renders a horizontal menu bar.

impl super::menubar::Menubar {
    /// Shows the menu bar. Content closure should add menu items.
    pub fn show(self, ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let frame = egui::Frame::NONE
            .fill(theme.background)
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 0,
                bottom: 0,
            })
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgba_unmultiplied(
                    theme.border.r(),
                    theme.border.g(),
                    theme.border.b(),
                    theme.border.a(),
                ),
            ));

        frame
            .show(ui, |ui| {
                ui.set_min_height(36.0);
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    content(ui);
                });
            })
            .response
    }

    /// Creates a single menu bar item (trigger button for a submenu).
    pub fn item(ui: &mut egui::Ui, label: &str) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());

        let galley = ui.painter().layout_no_wrap(
            label.to_owned(),
            egui::FontId::proportional(13.0),
            theme.foreground,
        );

        let h_pad: f32 = 8.0;
        let desired = egui::vec2(galley.size().x + h_pad * 2.0, 28.0);
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let cr = egui::CornerRadius::same(4);
            if response.is_pointer_button_down_on() {
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
            ui.painter().galley(text_pos, galley, theme.foreground);
        }

        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response
    }

    /// Creates a menu bar item with a dropdown submenu.
    /// `items` is a list of submenu item labels.
    /// `on_select` is called with the index of the clicked item.
    pub fn menu(ui: &mut egui::Ui, label: &str, items: &[&str], on_select: impl FnOnce(usize)) {
        let trigger = Self::item(ui, label);
        crate::widgets::dropdown_menu::dropdown_menu::DropdownMenu::show(
            ui, &trigger, items, on_select,
        );
    }
}
