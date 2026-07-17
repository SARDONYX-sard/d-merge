//! Paints a Lucide icon using the egui `Painter`.

/// Paint a Lucide icon into `rect` with the given `color`.
///
/// The icon is rendered as vector strokes (not rasterised), matching the
/// Lucide default: stroke-width 2, round linecap/linejoin, 24×24 viewBox.
pub fn paint_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: &super::lucide_icon::LucideIcon,
    color: egui::Color32,
) {
    paint_icon_svg(painter, rect, icon.svg_data(), color);
}

/// Paint a raw Lucide-style SVG body (the inner `<path>`/`<circle>`/… elements,
/// 24×24 viewBox, no `<svg>` wrapper) into `rect` with the given `color`.
///
/// Use this for custom icons authored in the Lucide grammar that are not part of
/// the [`LucideIcon`](super::lucide_icon::LucideIcon) set.
pub fn paint_icon_svg(
    painter: &egui::Painter,
    rect: egui::Rect,
    svg_body: &str,
    color: egui::Color32,
) {
    let elements = super::parse_svg::parse_svg(svg_body);
    let scale = rect.width().min(rect.height()) / 24.0;
    let offset = rect.min;
    let stroke = egui::Stroke::new(2.0 * scale, color);

    for element in &elements {
        paint_element(painter, element, scale, offset, stroke);
    }
}

fn paint_element(
    painter: &egui::Painter,
    element: &super::icon_element::IconElement,
    scale: f32,
    offset: egui::Pos2,
    stroke: egui::Stroke,
) {
    match element {
        super::icon_element::IconElement::Path(commands) => {
            paint_path(painter, commands, scale, offset, stroke);
        }
        super::icon_element::IconElement::Circle { cx, cy, r } => {
            let center = egui::pos2(cx * scale + offset.x, cy * scale + offset.y);
            painter.circle_stroke(center, r * scale, stroke);
        }
        super::icon_element::IconElement::Rect {
            x,
            y,
            width,
            height,
            rx,
        } => {
            let r = egui::Rect::from_min_size(
                egui::pos2(x * scale + offset.x, y * scale + offset.y),
                egui::vec2(width * scale, height * scale),
            );
            let cr = egui::CornerRadius::same((rx * scale) as u8);
            painter.rect_stroke(r, cr, stroke, egui::epaint::StrokeKind::Outside);
        }
        super::icon_element::IconElement::Line { x1, y1, x2, y2 } => {
            let p1 = egui::pos2(x1 * scale + offset.x, y1 * scale + offset.y);
            let p2 = egui::pos2(x2 * scale + offset.x, y2 * scale + offset.y);
            painter.line_segment([p1, p2], stroke);
        }
        super::icon_element::IconElement::Polyline(points) => {
            let pts: Vec<egui::Pos2> = points
                .iter()
                .map(|(x, y)| egui::pos2(x * scale + offset.x, y * scale + offset.y))
                .collect();
            if pts.len() >= 2 {
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
        super::icon_element::IconElement::Polygon(points) => {
            let pts: Vec<egui::Pos2> = points
                .iter()
                .map(|(x, y)| egui::pos2(x * scale + offset.x, y * scale + offset.y))
                .collect();
            if pts.len() >= 2 {
                painter.add(egui::Shape::closed_line(pts, stroke));
            }
        }
        super::icon_element::IconElement::Ellipse { cx, cy, rx, ry } => {
            paint_ellipse(painter, *cx, *cy, *rx, *ry, scale, offset, stroke);
        }
    }
}

// ── Path tessellation ───────────────────────────────────────────

fn paint_path(
    painter: &egui::Painter,
    commands: &[super::path_command::PathCommand],
    scale: f32,
    offset: egui::Pos2,
    stroke: egui::Stroke,
) {
    let mut points: Vec<egui::Pos2> = Vec::new();
    let mut cx: f32 = 0.0;
    let mut cy: f32 = 0.0;
    let mut subpath_start_x: f32 = 0.0;
    let mut subpath_start_y: f32 = 0.0;
    // For smooth cubic/quad continuations
    let mut last_ctrl_x: f32 = 0.0;
    let mut last_ctrl_y: f32 = 0.0;
    let mut last_was_cubic = false;
    let mut last_was_quad = false;

    for cmd in commands {
        match cmd {
            super::path_command::PathCommand::MoveToAbs(x, y) => {
                flush_subpath(painter, &mut points, stroke, false);
                cx = *x;
                cy = *y;
                subpath_start_x = cx;
                subpath_start_y = cy;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::MoveToRel(dx, dy) => {
                flush_subpath(painter, &mut points, stroke, false);
                cx += dx;
                cy += dy;
                subpath_start_x = cx;
                subpath_start_y = cy;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::LineToAbs(x, y) => {
                cx = *x;
                cy = *y;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::LineToRel(dx, dy) => {
                cx += dx;
                cy += dy;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::HorizontalAbs(x) => {
                cx = *x;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::HorizontalRel(dx) => {
                cx += dx;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::VerticalAbs(y) => {
                cy = *y;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::VerticalRel(dy) => {
                cy += dy;
                points.push(map_point(cx, cy, scale, offset));
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::CubicAbs(x1, y1, x2, y2, x, y) => {
                tessellate_cubic(
                    &mut points, cx, cy, *x1, *y1, *x2, *y2, *x, *y, scale, offset,
                );
                last_ctrl_x = *x2;
                last_ctrl_y = *y2;
                cx = *x;
                cy = *y;
                last_was_cubic = true;
                last_was_quad = false;
            }
            super::path_command::PathCommand::CubicRel(dx1, dy1, dx2, dy2, dx, dy) => {
                let (x1, y1) = (cx + dx1, cy + dy1);
                let (x2, y2) = (cx + dx2, cy + dy2);
                let (ex, ey) = (cx + dx, cy + dy);
                tessellate_cubic(&mut points, cx, cy, x1, y1, x2, y2, ex, ey, scale, offset);
                last_ctrl_x = x2;
                last_ctrl_y = y2;
                cx = ex;
                cy = ey;
                last_was_cubic = true;
                last_was_quad = false;
            }
            super::path_command::PathCommand::SmoothCubicAbs(x2, y2, x, y) => {
                let (x1, y1) = if last_was_cubic {
                    (2.0 * cx - last_ctrl_x, 2.0 * cy - last_ctrl_y)
                } else {
                    (cx, cy)
                };
                tessellate_cubic(
                    &mut points, cx, cy, x1, y1, *x2, *y2, *x, *y, scale, offset,
                );
                last_ctrl_x = *x2;
                last_ctrl_y = *y2;
                cx = *x;
                cy = *y;
                last_was_cubic = true;
                last_was_quad = false;
            }
            super::path_command::PathCommand::SmoothCubicRel(dx2, dy2, dx, dy) => {
                let (x1, y1) = if last_was_cubic {
                    (2.0 * cx - last_ctrl_x, 2.0 * cy - last_ctrl_y)
                } else {
                    (cx, cy)
                };
                let (x2, y2) = (cx + dx2, cy + dy2);
                let (ex, ey) = (cx + dx, cy + dy);
                tessellate_cubic(&mut points, cx, cy, x1, y1, x2, y2, ex, ey, scale, offset);
                last_ctrl_x = x2;
                last_ctrl_y = y2;
                cx = ex;
                cy = ey;
                last_was_cubic = true;
                last_was_quad = false;
            }
            super::path_command::PathCommand::QuadAbs(x1, y1, x, y) => {
                tessellate_quad(&mut points, cx, cy, *x1, *y1, *x, *y, scale, offset);
                last_ctrl_x = *x1;
                last_ctrl_y = *y1;
                cx = *x;
                cy = *y;
                last_was_quad = true;
                last_was_cubic = false;
            }
            super::path_command::PathCommand::QuadRel(dx1, dy1, dx, dy) => {
                let (x1, y1) = (cx + dx1, cy + dy1);
                let (ex, ey) = (cx + dx, cy + dy);
                tessellate_quad(&mut points, cx, cy, x1, y1, ex, ey, scale, offset);
                last_ctrl_x = x1;
                last_ctrl_y = y1;
                cx = ex;
                cy = ey;
                last_was_quad = true;
                last_was_cubic = false;
            }
            super::path_command::PathCommand::SmoothQuadAbs(x, y) => {
                let (x1, y1) = if last_was_quad {
                    (2.0 * cx - last_ctrl_x, 2.0 * cy - last_ctrl_y)
                } else {
                    (cx, cy)
                };
                tessellate_quad(&mut points, cx, cy, x1, y1, *x, *y, scale, offset);
                last_ctrl_x = x1;
                last_ctrl_y = y1;
                cx = *x;
                cy = *y;
                last_was_quad = true;
                last_was_cubic = false;
            }
            super::path_command::PathCommand::SmoothQuadRel(dx, dy) => {
                let (x1, y1) = if last_was_quad {
                    (2.0 * cx - last_ctrl_x, 2.0 * cy - last_ctrl_y)
                } else {
                    (cx, cy)
                };
                let (ex, ey) = (cx + dx, cy + dy);
                tessellate_quad(&mut points, cx, cy, x1, y1, ex, ey, scale, offset);
                last_ctrl_x = x1;
                last_ctrl_y = y1;
                cx = ex;
                cy = ey;
                last_was_quad = true;
                last_was_cubic = false;
            }
            super::path_command::PathCommand::ArcAbs {
                rx,
                ry,
                angle,
                large_arc,
                sweep,
                x,
                y,
            } => {
                tessellate_arc(
                    &mut points,
                    cx,
                    cy,
                    *rx,
                    *ry,
                    *angle,
                    *large_arc,
                    *sweep,
                    *x,
                    *y,
                    scale,
                    offset,
                );
                cx = *x;
                cy = *y;
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::ArcRel {
                rx,
                ry,
                angle,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let (ex, ey) = (cx + x, cy + y);
                tessellate_arc(
                    &mut points, cx, cy, *rx, *ry, *angle, *large_arc, *sweep, ex, ey, scale,
                    offset,
                );
                cx = ex;
                cy = ey;
                last_was_cubic = false;
                last_was_quad = false;
            }
            super::path_command::PathCommand::Close => {
                cx = subpath_start_x;
                cy = subpath_start_y;
                flush_subpath(painter, &mut points, stroke, true);
            }
        }
    }

    flush_subpath(painter, &mut points, stroke, false);
}

fn map_point(x: f32, y: f32, scale: f32, offset: egui::Pos2) -> egui::Pos2 {
    egui::pos2(x * scale + offset.x, y * scale + offset.y)
}

fn flush_subpath(
    painter: &egui::Painter,
    points: &mut Vec<egui::Pos2>,
    stroke: egui::Stroke,
    closed: bool,
) {
    if points.len() >= 2 {
        let pts = std::mem::take(points);
        if closed {
            painter.add(egui::Shape::closed_line(pts, stroke));
        } else {
            painter.add(egui::Shape::line(pts, stroke));
        }
    } else {
        points.clear();
    }
}

// ── Bézier tessellation ─────────────────────────────────────────

const CUBIC_SEGMENTS: usize = 8;
const QUAD_SEGMENTS: usize = 6;

fn tessellate_cubic(
    points: &mut Vec<egui::Pos2>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
    scale: f32,
    offset: egui::Pos2,
) {
    for i in 1..=CUBIC_SEGMENTS {
        let t = i as f32 / CUBIC_SEGMENTS as f32;
        let u = 1.0 - t;
        let x = u * u * u * x0 + 3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t * x3;
        let y = u * u * u * y0 + 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t * y3;
        points.push(map_point(x, y, scale, offset));
    }
}

fn tessellate_quad(
    points: &mut Vec<egui::Pos2>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    scale: f32,
    offset: egui::Pos2,
) {
    for i in 1..=QUAD_SEGMENTS {
        let t = i as f32 / QUAD_SEGMENTS as f32;
        let u = 1.0 - t;
        let x = u * u * x0 + 2.0 * u * t * x1 + t * t * x2;
        let y = u * u * y0 + 2.0 * u * t * y1 + t * t * y2;
        points.push(map_point(x, y, scale, offset));
    }
}

// ── Arc to Bézier ───────────────────────────────────────────────

/// Convert an SVG arc to one or more cubic Bézier segments.
///
/// Follows the SVG spec endpoint-to-center parameterisation.
fn tessellate_arc(
    points: &mut Vec<egui::Pos2>,
    x1: f32,
    y1: f32,
    mut rx: f32,
    mut ry: f32,
    x_rotation_deg: f32,
    large_arc: bool,
    sweep: bool,
    x2: f32,
    y2: f32,
    scale: f32,
    offset: egui::Pos2,
) {
    // Degenerate: zero radius → line
    if rx.abs() < 1e-6 || ry.abs() < 1e-6 {
        points.push(map_point(x2, y2, scale, offset));
        return;
    }

    rx = rx.abs();
    ry = ry.abs();
    let phi = x_rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    // Step 1: compute (x1', y1') in rotated coordinate system
    let dx2 = (x1 - x2) / 2.0;
    let dy2 = (y1 - y2) / 2.0;
    let x1p = cos_phi * dx2 + sin_phi * dy2;
    let y1p = -sin_phi * dx2 + cos_phi * dy2;

    // Step 2: correct radii if too small
    let x1p2 = x1p * x1p;
    let y1p2 = y1p * y1p;
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let lambda = x1p2 / rx2 + y1p2 / ry2;
    if lambda > 1.0 {
        let sqrt_lambda = lambda.sqrt();
        rx *= sqrt_lambda;
        ry *= sqrt_lambda;
    }
    let rx2 = rx * rx;
    let ry2 = ry * ry;

    // Step 3: compute center point (cx', cy')
    let num = (rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2).max(0.0);
    let den = rx2 * y1p2 + ry2 * x1p2;
    let sq = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
    let sign = if large_arc == sweep { -1.0 } else { 1.0 };
    let cxp = sign * sq * (rx * y1p / ry);
    let cyp = sign * sq * (-(ry * x1p / rx));

    // Step 4: compute center (cx, cy) in original coordinates
    let cx = cos_phi * cxp - sin_phi * cyp + (x1 + x2) / 2.0;
    let cy = sin_phi * cxp + cos_phi * cyp + (y1 + y2) / 2.0;

    // Step 5: compute start angle and sweep angle
    let theta1 = angle_between(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dtheta = angle_between(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );

    if !sweep && dtheta > 0.0 {
        dtheta -= std::f32::consts::TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += std::f32::consts::TAU;
    }

    // Step 6: approximate arc with cubic Bézier segments
    let n_segs = ((dtheta.abs() / (std::f32::consts::FRAC_PI_4)).ceil() as usize).max(1);
    let seg_angle = dtheta / n_segs as f32;

    for i in 0..n_segs {
        let a1 = theta1 + seg_angle * i as f32;
        let a2 = a1 + seg_angle;
        arc_segment_to_cubic(points, cx, cy, rx, ry, sin_phi, cos_phi, a1, a2, scale, offset);
    }
}

fn arc_segment_to_cubic(
    points: &mut Vec<egui::Pos2>,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    sin_phi: f32,
    cos_phi: f32,
    a1: f32,
    a2: f32,
    scale: f32,
    offset: egui::Pos2,
) {
    let half = (a2 - a1) / 2.0;
    let alpha = half.sin() * ((4.0 + 3.0 * (2.0 * half).tan().powi(2)).sqrt() - 1.0) / 3.0;

    let (sin1, cos1) = a1.sin_cos();
    let (sin2, cos2) = a2.sin_cos();

    let ex1 = rx * cos1;
    let ey1 = ry * sin1;
    let ex2 = rx * cos2;
    let ey2 = ry * sin2;

    // Control point 1
    let dx1 = -rx * sin1;
    let dy1 = ry * cos1;
    let cp1x = cx + cos_phi * (ex1 + alpha * dx1) - sin_phi * (ey1 + alpha * dy1);
    let cp1y = cy + sin_phi * (ex1 + alpha * dx1) + cos_phi * (ey1 + alpha * dy1);

    // Control point 2
    let dx2 = -rx * sin2;
    let dy2 = ry * cos2;
    let cp2x = cx + cos_phi * (ex2 - alpha * dx2) - sin_phi * (ey2 - alpha * dy2);
    let cp2y = cy + sin_phi * (ex2 - alpha * dx2) + cos_phi * (ey2 - alpha * dy2);

    // End point
    let px = cx + cos_phi * ex2 - sin_phi * ey2;
    let py = cy + sin_phi * ex2 + cos_phi * ey2;

    // Tessellate this cubic segment
    let prev_x = cx + cos_phi * ex1 - sin_phi * ey1;
    let prev_y = cy + sin_phi * ex1 + cos_phi * ey1;
    tessellate_cubic(points, prev_x, prev_y, cp1x, cp1y, cp2x, cp2y, px, py, scale, offset);
}

fn angle_between(ux: f32, uy: f32, vx: f32, vy: f32) -> f32 {
    let dot = ux * vx + uy * vy;
    let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
    let cos_val = (dot / len).clamp(-1.0, 1.0);
    let angle = cos_val.acos();
    if ux * vy - uy * vx < 0.0 {
        -angle
    } else {
        angle
    }
}

// ── Ellipse approximation ───────────────────────────────────────

fn paint_ellipse(
    painter: &egui::Painter,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    scale: f32,
    offset: egui::Pos2,
    stroke: egui::Stroke,
) {
    const N: usize = 32;
    let pts: Vec<egui::Pos2> = (0..N)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / N as f32;
            let x = cx + rx * angle.cos();
            let y = cy + ry * angle.sin();
            map_point(x, y, scale, offset)
        })
        .collect();

    painter.add(egui::Shape::closed_line(pts, stroke));
}
