//! Shared focus ring painting used by all focusable widgets.

/// Paints a 3px focus ring around `rect` with the given corner radius and color.
pub fn paint_focus_ring(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: f32,
    ring_color: egui::Color32,
) {
    let ring_rect = rect.expand(2.0);
    let ring_alpha = 128;
    let ring_c = egui::Color32::from_rgba_unmultiplied(
        ring_color.r(),
        ring_color.g(),
        ring_color.b(),
        ring_alpha,
    );
    let cr = (corner_radius + 2.0).round() as u8;
    painter.rect_stroke(
        ring_rect,
        egui::CornerRadius::same(cr),
        egui::Stroke::new(3.0, ring_c),
        egui::epaint::StrokeKind::Outside,
    );
}
