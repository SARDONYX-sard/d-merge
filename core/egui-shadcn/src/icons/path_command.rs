//! SVG path `d`-attribute commands.

/// A single command from an SVG path `d` attribute.
#[allow(clippy::upper_case_acronyms)]
pub enum PathCommand {
    // ── Move ────────────────────────────────────────────────────
    MoveToAbs(f32, f32),
    MoveToRel(f32, f32),

    // ── Line ────────────────────────────────────────────────────
    LineToAbs(f32, f32),
    LineToRel(f32, f32),
    HorizontalAbs(f32),
    HorizontalRel(f32),
    VerticalAbs(f32),
    VerticalRel(f32),

    // ── Cubic Bézier ────────────────────────────────────────────
    /// C x1 y1 x2 y2 x y
    CubicAbs(f32, f32, f32, f32, f32, f32),
    /// c dx1 dy1 dx2 dy2 dx dy
    CubicRel(f32, f32, f32, f32, f32, f32),
    /// S x2 y2 x y
    SmoothCubicAbs(f32, f32, f32, f32),
    /// s dx2 dy2 dx dy
    SmoothCubicRel(f32, f32, f32, f32),

    // ── Quadratic Bézier ────────────────────────────────────────
    /// Q x1 y1 x y
    QuadAbs(f32, f32, f32, f32),
    /// q dx1 dy1 dx dy
    QuadRel(f32, f32, f32, f32),
    /// T x y
    SmoothQuadAbs(f32, f32),
    /// t dx dy
    SmoothQuadRel(f32, f32),

    // ── Arc ─────────────────────────────────────────────────────
    ArcAbs {
        rx: f32,
        ry: f32,
        angle: f32,
        large_arc: bool,
        sweep: bool,
        x: f32,
        y: f32,
    },
    ArcRel {
        rx: f32,
        ry: f32,
        angle: f32,
        large_arc: bool,
        sweep: bool,
        x: f32,
        y: f32,
    },

    // ── Close ───────────────────────────────────────────────────
    Close,
}
