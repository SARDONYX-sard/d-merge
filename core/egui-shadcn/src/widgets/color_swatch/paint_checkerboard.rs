//! Checkerboard painter for transparent color swatches.

pub(crate) fn paint_checkerboard(painter: &egui::Painter, rect: egui::Rect, cell: f32) {
    let light = egui::Color32::from_gray(210);
    let dark = egui::Color32::from_gray(150);
    let cols = (rect.width() / cell).ceil() as usize;
    let rows = (rect.height() / cell).ceil() as usize;

    for row in 0..rows {
        for col in 0..cols {
            let min = egui::pos2(
                rect.min.x + col as f32 * cell,
                rect.min.y + row as f32 * cell,
            );
            let max = egui::pos2(
                (min.x + cell).min(rect.max.x),
                (min.y + cell).min(rect.max.y),
            );
            let color = if (row + col) % 2 == 0 { light } else { dark };
            painter.rect_filled(egui::Rect::from_min_max(min, max), 0.0, color);
        }
    }
}
