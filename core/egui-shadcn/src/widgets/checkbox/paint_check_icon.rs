//! Paints the checkmark polyline inside a checkbox.

/// Draws a checkmark icon within `rect`, scaled by `anim_t` (0..1).
pub fn paint_check_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    anim_t: f32,
) {
    if anim_t < 0.01 {
        return;
    }

    let cx = rect.center().x;
    let cy = rect.center().y;
    let s = rect.width() * 0.3 * anim_t;

    let points = vec![
        egui::pos2(cx - s, cy),
        egui::pos2(cx - s * 0.25, cy + s * 0.7),
        egui::pos2(cx + s, cy - s * 0.5),
    ];

    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, color),
    ));
}
