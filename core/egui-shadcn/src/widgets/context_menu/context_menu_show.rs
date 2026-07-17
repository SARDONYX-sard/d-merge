//! Show method for ContextMenu — renders a right-click popup.

impl super::context_menu::ContextMenu {
    /// Attaches a context menu to `response`. Shows on right-click.
    /// Calls `on_select(index)` when an item is clicked.
    pub fn show(
        response: &egui::Response,
        items: &[&str],
        on_select: impl FnOnce(usize),
    ) {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(&response.ctx);
        let mut selected_idx = None;

        response.context_menu(|ui| {
            // Compute max text width for a tight menu
            let max_text_width: f32 = items
                .iter()
                .map(|label| {
                    ui.painter()
                        .layout_no_wrap(
                            label.to_string(),
                            egui::FontId::proportional(14.0),
                            theme.popover_foreground,
                        )
                        .size()
                        .x
                })
                .fold(0.0_f32, f32::max);

            let menu_width = (max_text_width + 24.0).max(120.0);
            ui.set_min_width(menu_width);
            ui.set_max_width(menu_width);

            for (idx, &label) in items.iter().enumerate() {
                let galley = ui.painter().layout_no_wrap(
                    label.to_owned(),
                    egui::FontId::proportional(14.0),
                    theme.popover_foreground,
                );
                let desired = egui::vec2(menu_width, galley.size().y + 8.0);
                let (rect, r) = ui.allocate_exact_size(desired, egui::Sense::click());

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
                    selected_idx = Some(idx);
                    ui.close();
                }
            }
        });

        if let Some(idx) = selected_idx {
            on_select(idx);
        }
    }
}
