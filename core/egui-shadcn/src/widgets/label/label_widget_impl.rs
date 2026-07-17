//! Widget trait implementation for Label.

impl egui::Widget for super::label::Label {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let color = if self.muted {
            theme.muted_foreground
        } else {
            theme.foreground
        };

        let (font_size, fixed_height) = match self.size {
            Some(size) => {
                let (height, _padding, fs) = size.metrics();
                (fs, Some(height))
            }
            None => (14.0, None),
        };

        let galley = ui.painter().layout_no_wrap(
            self.text,
            egui::FontId::proportional(font_size),
            color,
        );

        let desired = match fixed_height {
            Some(h) => egui::vec2(galley.size().x, h),
            None => galley.size(),
        };
        let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // Center text vertically within the allocated rect (matches button centering)
            let text_pos = egui::pos2(
                rect.min.x,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, color);
        }

        response
    }
}
