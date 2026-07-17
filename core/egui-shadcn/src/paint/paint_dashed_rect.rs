//! Dashed rectangle painting utility.

/// Paints a dashed border rectangle.
pub fn paint_dashed_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    corner_radius: f32,
    stroke: egui::Stroke,
    dash_length: f32,
    gap_length: f32,
) {
    // Approximate with small line segments along each edge.
    let edges: [(egui::Pos2, egui::Pos2); 4] = [
        (rect.left_top(), rect.right_top()),     // top
        (rect.right_top(), rect.right_bottom()),  // right
        (rect.right_bottom(), rect.left_bottom()),// bottom
        (rect.left_bottom(), rect.left_top()),    // left
    ];

    let _ = corner_radius; // corner rounding not applied to dash segments

    for (start, end) in edges {
        let dir = end - start;
        let length = dir.length();
        if length < 0.1 {
            continue;
        }
        let unit = dir / length;
        let mut cursor = 0.0_f32;
        let mut drawing = true;

        while cursor < length {
            let seg_len = if drawing { dash_length } else { gap_length };
            let seg_end = (cursor + seg_len).min(length);

            if drawing {
                let p0 = start + unit * cursor;
                let p1 = start + unit * seg_end;
                painter.line_segment([p0, p1], stroke);
            }

            cursor = seg_end;
            drawing = !drawing;
        }
    }
}
