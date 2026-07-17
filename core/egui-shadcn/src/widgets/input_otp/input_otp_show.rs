//! Show method for InputOtp — renders individual digit input boxes.

impl super::input_otp::InputOtp {
    /// Shows the OTP input. `value` is a mutable string holding the entered digits.
    pub fn show(self, ui: &mut egui::Ui, value: &mut String) -> egui::Response {
        let theme = crate::theme::shadcn_theme_ext::ShadcnThemeExt::shadcn_theme(ui.ctx());
        let cell_size: f32 = 40.0;
        let gap: f32 = 8.0;
        let font_size: f32 = 20.0;
        let cr = egui::CornerRadius::same(theme.radius.round() as u8);

        let total_width = cell_size * self.length as f32 + gap * (self.length - 1) as f32;
        let (full_rect, response) =
            ui.allocate_exact_size(egui::vec2(total_width, cell_size), egui::Sense::click());

        // Handle keyboard input when focused
        if response.has_focus() || response.clicked() {
            response.request_focus();

            let events: Vec<egui::Event> = ui.ctx().input(|i| i.events.clone());
            for event in &events {
                match event {
                    egui::Event::Text(t) => {
                        for ch in t.chars() {
                            if ch.is_ascii_digit() && value.len() < self.length {
                                value.push(ch);
                            }
                        }
                        ui.ctx().request_repaint();
                    }
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => {
                        value.pop();
                        ui.ctx().request_repaint();
                    }
                    _ => {}
                }
            }
        }

        if ui.is_rect_visible(full_rect) {
            let painter = ui.painter();
            let updated_digits: Vec<char> = value.chars().collect();

            for i in 0..self.length {
                let x = full_rect.min.x + (cell_size + gap) * i as f32;
                let cell_rect = egui::Rect::from_min_size(
                    egui::pos2(x, full_rect.min.y),
                    egui::vec2(cell_size, cell_size),
                );

                let is_active = i == updated_digits.len() && response.has_focus();
                let border_color = if is_active { theme.ring } else { theme.input };

                painter.rect_filled(cell_rect, cr, theme.background);
                painter.rect_stroke(
                    cell_rect,
                    cr,
                    egui::Stroke::new(if is_active { 2.0 } else { 1.0 }, border_color),
                    egui::epaint::StrokeKind::Inside,
                );

                if let Some(&ch) = updated_digits.get(i) {
                    let galley = painter.layout_no_wrap(
                        ch.to_string(),
                        egui::FontId::proportional(font_size),
                        theme.foreground,
                    );
                    let pos = egui::pos2(
                        cell_rect.center().x - galley.size().x / 2.0,
                        cell_rect.center().y - galley.size().y / 2.0,
                    );
                    painter.galley(pos, galley, theme.foreground);
                }
            }

            // Focus ring
            if response.has_focus() {
                crate::paint::paint_focus_ring::paint_focus_ring(
                    painter,
                    full_rect,
                    theme.radius,
                    theme.ring,
                );
            }
        }

        response
    }
}
