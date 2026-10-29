// SPDX-FileCopyrightText: (C) 2026 Pawel Jankiewicz, SARDONYX
// SPDX-License-Identifier: MIT
// ref: https://github.com/pjankiewicz/egui-shadcn/tree/fa5ceeed623eea983765fa4f886dd610d8b39470/src/icons
//! Lucide icons rendered as vector strokes.
//!
//! Only the icons used by this app are embedded. Supported SVG subset: `<path>`(`M L H V C A Z`),
//! `<circle>` and `<rect>`.

use egui::{Pos2, pos2};

/// Lucide icons (ISC License: https://lucide.dev/license).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Check,
    ChevronDown,
    FolderSearch,
    Layers,
    Lock,
    Logs,
    RefreshCcw,
    RefreshCwOff,
    ScanSearch,
    Search,
    TextAlignJustify,
    Toolbox,
}

impl Icon {
    /// Inner SVG XML (elements inside the 24x24 `<svg>` wrapper).
    const fn svg_data(self) -> &'static str {
        match self {
            Self::Check => r#"<path d="M20 6 9 17l-5-5" />"#,
            Self::ChevronDown => r#"<path d="m6 9 6 6 6-6" />"#,
            Self::FolderSearch => {
                r#"<path d="M10.7 20H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H20a2 2 0 0 1 2 2v4.1" /> <path d="m21 21-1.9-1.9" /> <circle cx="17" cy="17" r="3" />"#
            }
            Self::Layers => {
                r#"<path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z" /> <path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12" /> <path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17" />"#
            }
            Self::Lock => {
                r#"<rect width="18" height="11" x="3" y="11" rx="2" ry="2" /> <path d="M7 11V7a5 5 0 0 1 10 0v4" />"#
            }
            Self::Logs => {
                r#"<path d="M3 5h1" /> <path d="M3 12h1" /> <path d="M3 19h1" /> <path d="M8 5h1" /> <path d="M8 12h1" /> <path d="M8 19h1" /> <path d="M13 5h8" /> <path d="M13 12h8" /> <path d="M13 19h8" />"#
            }
            Self::RefreshCcw => {
                r#"<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /> <path d="M3 3v5h5" /> <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" /> <path d="M16 16h5v5" />"#
            }
            Self::RefreshCwOff => {
                r#"<path d="M21 8L18.74 5.74A9.75 9.75 0 0 0 12 3C11 3 10.03 3.16 9.13 3.47" /> <path d="M8 16H3v5" /> <path d="M3 12C3 9.51 4 7.26 5.64 5.64" /> <path d="m3 16 2.26 2.26A9.75 9.75 0 0 0 12 21c2.49 0 4.74-1 6.36-2.64" /> <path d="M21 12c0 1-.16 1.97-.47 2.87" /> <path d="M21 3v5h-5" /> <path d="M22 22 2 2" />"#
            }
            Self::ScanSearch => {
                r#"<path d="M3 7V5a2 2 0 0 1 2-2h2" /> <path d="M17 3h2a2 2 0 0 1 2 2v2" /> <path d="M21 17v2a2 2 0 0 1-2 2h-2" /> <path d="M7 21H5a2 2 0 0 1-2-2v-2" /> <circle cx="12" cy="12" r="3" /> <path d="m16 16-1.9-1.9" />"#
            }
            Self::Search => r#"<path d="m21 21-4.34-4.34" /> <circle cx="11" cy="11" r="8" />"#,
            Self::TextAlignJustify => {
                r#"<path d="M3 5h18" /> <path d="M3 12h18" /> <path d="M3 19h18" />"#
            }
            Self::Toolbox => {
                r#"<path d="M16 12v4" /> <path d="M16 6a2 2 0 0 1 1.414.586l4 4A2 2 0 0 1 22 12v7a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 .586-1.414l4-4A2 2 0 0 1 8 6z" /> <path d="M16 6V4a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v2" /> <path d="M2 14h20" /> <path d="M8 12v4" />"#
            }
        }
    }
}

/// Shows the icon as an 18px image in the foreground color: `ui.add(Icon::Search)`.
impl egui::Widget for Icon {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(18.0), egui::Sense::hover());
        let color = super::theme::ShadcnTheme::get(ui.ctx()).foreground;
        paint_icon(ui.painter(), rect, self, color);
        response
    }
}

/// Paint an icon into `rect` with the given `color`.
///
/// Matches the Lucide default: stroke-width 2, 24×24 viewBox.
pub(crate) fn paint_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: Icon,
    color: egui::Color32,
) {
    let scale = rect.width().min(rect.height()) / 24.0;
    let mut pen = Pen {
        painter,
        stroke: egui::Stroke::new(2.0 * scale, color),
        scale,
        offset: rect.min,
        points: Vec::new(),
        cur: Pos2::ZERO,
        start: Pos2::ZERO,
    };

    for chunk in icon.svg_data().split('<') {
        let tag = chunk.split('>').next().unwrap_or_default().trim().trim_end_matches('/');
        let num = |name: &str| attr(tag, name).and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);

        match tag.split_ascii_whitespace().next() {
            Some("path") => {
                if let Some(d) = attr(tag, "d") {
                    pen.path(d);
                }
            }
            Some("circle") => {
                let center = pen.map(pos2(num("cx"), num("cy")));
                painter.circle_stroke(center, num("r") * scale, pen.stroke);
            }
            Some("rect") => {
                let min = pen.map(pos2(num("x"), num("y")));
                let r =
                    egui::Rect::from_min_size(min, egui::vec2(num("width"), num("height")) * scale);
                let cr = egui::CornerRadius::same((num("rx") * scale) as u8);
                painter.rect_stroke(r, cr, pen.stroke, egui::epaint::StrokeKind::Outside);
            }
            _ => {}
        }
    }
}

/// Extract the value of ` name="value"` from a tag string.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(i) = rest.find(name) {
        let after = &rest[i + name.len()..];
        if rest[..i].ends_with(' ')
            && let Some(value) = after.strip_prefix("=\"")
        {
            return value.split('"').next();
        }
        rest = after;
    }
    None
}

// ── Path ────────────────────────────────────────────────────────

/// Path tessellation state.
struct Pen<'a> {
    painter: &'a egui::Painter,
    stroke: egui::Stroke,
    scale: f32,
    offset: Pos2,
    /// Current subpath in screen space.
    points: Vec<Pos2>,
    /// Current point in SVG space.
    cur: Pos2,
    /// Subpath start in SVG space.
    start: Pos2,
}

const CUBIC_SEGMENTS: usize = 8;

impl Pen<'_> {
    fn map(&self, p: Pos2) -> Pos2 {
        pos2(p.x * self.scale + self.offset.x, p.y * self.scale + self.offset.y)
    }

    fn line_to(&mut self, p: Pos2) {
        self.cur = p;
        self.points.push(self.map(p));
    }

    fn flush(&mut self, closed: bool) {
        if self.points.len() >= 2 {
            let pts = std::mem::take(&mut self.points);
            let shape = if closed {
                egui::Shape::closed_line(pts, self.stroke)
            } else {
                egui::Shape::line(pts, self.stroke)
            };
            self.painter.add(shape);
        } else {
            self.points.clear();
        }
    }

    /// Paint an SVG path `d` attribute.
    fn path(&mut self, d: &str) {
        let tokens = tokenize(d);
        let mut cmd = b'M';
        let mut i = 0;

        while i < tokens.len() {
            if let Token::Command(c) = tokens[i] {
                cmd = c;
                i += 1;
                if c.eq_ignore_ascii_case(&b'z') {
                    self.cur = self.start;
                    self.flush(true);
                    continue;
                }
            }

            let n = match cmd.to_ascii_uppercase() {
                b'H' | b'V' => 1,
                b'M' | b'L' => 2,
                b'C' => 6,
                b'A' => 7,
                _ => 0, // unsupported
            };
            let mut args = [0.0_f32; 7];
            let enough = n > 0
                && tokens.get(i..i + n).is_some_and(|ts| {
                    ts.iter().zip(&mut args).all(|(t, arg)| match t {
                        Token::Number(v) => {
                            *arg = *v;
                            true
                        }
                        Token::Command(_) => false,
                    })
                });
            if !enough {
                i += 1;
                continue;
            }
            i += n;

            self.command(cmd, &args);
            // After M, implicit repeats become L
            match cmd {
                b'M' => cmd = b'L',
                b'm' => cmd = b'l',
                _ => {}
            }
        }

        self.flush(false);
    }

    fn command(&mut self, cmd: u8, a: &[f32; 7]) {
        let o = if cmd.is_ascii_lowercase() { self.cur.to_vec2() } else { egui::Vec2::ZERO };
        let at = |x: f32, y: f32| pos2(x, y) + o;

        match cmd.to_ascii_uppercase() {
            b'M' => {
                self.flush(false);
                self.start = at(a[0], a[1]);
                self.line_to(self.start);
            }
            b'L' => self.line_to(at(a[0], a[1])),
            b'H' => self.line_to(pos2(a[0] + o.x, self.cur.y)),
            b'V' => self.line_to(pos2(self.cur.x, a[0] + o.y)),
            b'C' => self.cubic(at(a[0], a[1]), at(a[2], a[3]), at(a[4], a[5])),
            b'A' => self.arc(a[0], a[1], a[2], a[3] != 0.0, a[4] != 0.0, at(a[5], a[6])),
            _ => {}
        }
    }

    fn cubic(&mut self, p1: Pos2, p2: Pos2, p3: Pos2) {
        let p0 = self.cur;
        for i in 1..=CUBIC_SEGMENTS {
            let t = i as f32 / CUBIC_SEGMENTS as f32;
            let u = 1.0 - t;
            let p = p0.to_vec2() * (u * u * u)
                + p1.to_vec2() * (3.0 * u * u * t)
                + p2.to_vec2() * (3.0 * u * t * t)
                + p3.to_vec2() * (t * t * t);
            self.points.push(self.map(p.to_pos2()));
        }
        self.cur = p3;
    }

    /// Convert an SVG arc to cubic Bézier segments (SVG spec endpoint-to-center parameterisation).
    fn arc(
        &mut self,
        rx: f32,
        ry: f32,
        x_rotation_deg: f32,
        large_arc: bool,
        sweep: bool,
        end: Pos2,
    ) {
        let start = self.cur;

        // Degenerate: zero radius → line
        if rx.abs() < 1e-6 || ry.abs() < 1e-6 {
            self.line_to(end);
            return;
        }

        let (mut rx, mut ry) = (rx.abs(), ry.abs());
        let (sin_phi, cos_phi) = x_rotation_deg.to_radians().sin_cos();

        // Step 1: compute (x1', y1') in rotated coordinate system
        let d = (start - end) / 2.0;
        let x1p = cos_phi * d.x + sin_phi * d.y;
        let y1p = -sin_phi * d.x + cos_phi * d.y;

        // Step 2: correct radii if too small
        let (x1p2, y1p2) = (x1p * x1p, y1p * y1p);
        let lambda = x1p2 / (rx * rx) + y1p2 / (ry * ry);
        if lambda > 1.0 {
            let sqrt_lambda = lambda.sqrt();
            rx *= sqrt_lambda;
            ry *= sqrt_lambda;
        }
        let (rx2, ry2) = (rx * rx, ry * ry);

        // Step 3: compute center point (cx', cy')
        let num = (rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2).max(0.0);
        let den = rx2 * y1p2 + ry2 * x1p2;
        let sq = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
        let sign = if large_arc == sweep { -1.0 } else { 1.0 };
        let cxp = sign * sq * (rx * y1p / ry);
        let cyp = sign * sq * (-(ry * x1p / rx));

        // Step 4: compute center in original coordinates
        let mid = (start.to_vec2() + end.to_vec2()) / 2.0;
        let center =
            pos2(cos_phi * cxp - sin_phi * cyp + mid.x, sin_phi * cxp + cos_phi * cyp + mid.y);

        // Step 5: compute start angle and sweep angle
        let theta1 = angle_between(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
        let mut dtheta =
            angle_between((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
        if !sweep && dtheta > 0.0 {
            dtheta -= std::f32::consts::TAU;
        } else if sweep && dtheta < 0.0 {
            dtheta += std::f32::consts::TAU;
        }

        // Step 6: approximate arc with cubic Bézier segments
        let n_segs = ((dtheta.abs() / std::f32::consts::FRAC_PI_4).ceil() as usize).max(1);
        let seg_angle = dtheta / n_segs as f32;

        // Point on the rotated ellipse at angle `a`, offset by `alpha` along its tangent.
        let ellipse = |a: f32, alpha: f32| {
            let (sin, cos) = a.sin_cos();
            let ex = rx * cos + alpha * -rx * sin;
            let ey = ry * sin + alpha * ry * cos;
            pos2(center.x + cos_phi * ex - sin_phi * ey, center.y + sin_phi * ex + cos_phi * ey)
        };

        for i in 0..n_segs {
            let a1 = theta1 + seg_angle * i as f32;
            let a2 = a1 + seg_angle;
            let half = (a2 - a1) / 2.0;
            let alpha = half.sin() * ((4.0 + 3.0 * (2.0 * half).tan().powi(2)).sqrt() - 1.0) / 3.0;

            self.cur = ellipse(a1, 0.0);
            self.cubic(ellipse(a1, alpha), ellipse(a2, -alpha), ellipse(a2, 0.0));
        }
        self.cur = end;
    }
}

fn angle_between(ux: f32, uy: f32, vx: f32, vy: f32) -> f32 {
    let dot = ux * vx + uy * vy;
    let len = ux.hypot(uy) * vx.hypot(vy);
    let angle = (dot / len).clamp(-1.0, 1.0).acos();
    if ux * vy - uy * vx < 0.0 { -angle } else { angle }
}

// ── Tokenizer ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum Token {
    Command(u8),
    Number(f32),
}

fn tokenize(d: &str) -> Vec<Token> {
    let bytes = d.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let ch = bytes[i];

        if ch.is_ascii_alphabetic() {
            tokens.push(Token::Command(ch));
            i += 1;
            continue;
        }

        if matches!(ch, b'-' | b'+' | b'.') || ch.is_ascii_digit() {
            let start = i;
            let skip_digits = |mut i: usize| {
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                i
            };

            // Leading sign
            if matches!(ch, b'-' | b'+') {
                i += 1;
            }
            // Integer part
            i = skip_digits(i);
            // Decimal part
            if bytes.get(i) == Some(&b'.') {
                i = skip_digits(i + 1);
            }

            if let Ok(v) = d[start..i].parse::<f32>() {
                tokens.push(Token::Number(v));
            }
            continue;
        }

        i += 1; // whitespace, ',' etc.
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_implicit_decimals() {
        // "a.5.5 0 0 1 .923 0" → a, .5, .5, 0, 0, 1, .923, 0
        let tokens = tokenize("a.5.5 0 0 1 .923 0L11-.16");
        let nums: Vec<f32> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Number(v) => Some(*v),
                Token::Command(_) => None,
            })
            .collect();
        assert_eq!(nums, [0.5, 0.5, 0.0, 0.0, 1.0, 0.923, 0.0, 11.0, -0.16]);
    }

    #[test]
    fn attr_requires_word_boundary() {
        let tag = r#"rect width="18" height="11" x="3" y="11" rx="2" ry="2""#;
        assert_eq!(attr(tag, "x"), Some("3"));
        assert_eq!(attr(tag, "rx"), Some("2"));
        assert_eq!(attr(tag, "r"), None);
    }
}
