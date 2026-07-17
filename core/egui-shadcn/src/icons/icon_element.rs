//! SVG icon elements parsed from Lucide icon data.

/// A single SVG element within an icon.
pub enum IconElement {
    /// `<path d="...">` — sequence of path commands.
    Path(Vec<super::path_command::PathCommand>),

    /// `<circle cx cy r>`.
    Circle { cx: f32, cy: f32, r: f32 },

    /// `<rect x y width height rx>`.
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rx: f32,
    },

    /// `<line x1 y1 x2 y2>`.
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
    },

    /// `<polyline points="...">` — open series of connected line segments.
    Polyline(Vec<(f32, f32)>),

    /// `<polygon points="...">` — closed series of connected line segments.
    Polygon(Vec<(f32, f32)>),

    /// `<ellipse cx cy rx ry>`.
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
    },
}
