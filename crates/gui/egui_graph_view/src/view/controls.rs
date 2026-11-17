//! The controls over the graph, for the mouse alone: the zoom (a slider between two magnifiers),
//! back to 100 %, the focus to the center, all the nodes in view, and how deep the graph goes.
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │ (-) ━━━━━●━━━━ (+)  100%   ⌖   ⛶   − 3 +               │  at the bottom left, over the graph
//! └────────────────────────────────────────────────────────┘
//! ```

use egui::{Pos2, Rect, Sense, Stroke, Vec2};

/// The hover texts of the controls, e.g. from the host's translations.
#[derive(Debug, Clone, Copy)]
pub struct GraphLabels<'a> {
    pub zoom_out: &'a str,
    pub zoom_in: &'a str,
    /// Of the zoom's `100%`: a click goes back to it.
    pub zoom_reset: &'a str,
    pub center: &'a str,
    pub fit: &'a str,
    /// Of the depth's `− 3 +`.
    pub depth: &'a str,
}

impl GraphLabels<'static> {
    pub const ENGLISH: Self = Self {
        zoom_out: "Zoom out",
        zoom_in: "Zoom in",
        zoom_reset: "Back to 100%",
        center: "Bring the focus to the center",
        fit: "Show all the nodes",
        depth: "How many layers right of the focus are shown",
    };
}

impl Default for GraphLabels<'static> {
    #[inline]
    fn default() -> Self {
        Self::ENGLISH
    }
}

/// What a control asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Control {
    /// To this zoom, around the middle of the view.
    Zoom(f32),
    Center,
    Fit,
    Depth(usize),
}

/// A zoom button's step.
const ZOOM_STEP: f32 = 1.25;
/// The size of an icon button.
const BUTTON: f32 = 22.0;
/// How deep the graph can go.
const DEPTH: std::ops::RangeInclusive<usize> = 1..=8;

/// Draws the controls at the bottom left of `view`. `(zoom, depth)`: as they are now.
pub(super) fn controls_ui(
    ui: &mut egui::Ui,
    view: Rect,
    labels: &GraphLabels<'_>,
    ((zoom, zoom_range), depth): ((f32, std::ops::RangeInclusive<f32>), usize),
) -> Option<Control> {
    let bar = Rect::from_min_size(view.left_bottom() + egui::vec2(8.0, -46.0), egui::vec2(380.0, 32.0));
    let builder = egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::left_to_right(egui::Align::Center));
    let mut asked = None;
    ui.scope_builder(builder, |ui| {
        let frame = egui::Frame::popup(ui.style()).inner_margin(egui::Margin::symmetric(6, 3));
        frame.show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            asked = zoom_ui(ui, labels, (zoom, zoom_range));
            ui.separator();
            if icon_button(ui, Glyph::Center, labels.center).clicked() {
                asked = Some(Control::Center);
            }
            if icon_button(ui, Glyph::Fit, labels.fit).clicked() {
                asked = Some(Control::Fit);
            }
            ui.separator();
            if let Some(depth) = depth_ui(ui, labels, depth) {
                asked = Some(Control::Depth(depth));
            }
        });
    });
    asked
}

/// `(-) slider (+) 100%`
fn zoom_ui(
    ui: &mut egui::Ui,
    labels: &GraphLabels<'_>,
    (zoom, range): (f32, std::ops::RangeInclusive<f32>),
) -> Option<Control> {
    let (min, max) = (*range.start(), *range.end());
    let zoom_out = icon_button(ui, Glyph::ZoomOut, labels.zoom_out).clicked();
    let mut asked = zoom_out.then(|| Control::Zoom((zoom / ZOOM_STEP).max(min)));
    let mut value = zoom;
    let slider = egui::Slider::new(&mut value, range).logarithmic(true).show_value(false);
    if ui.add_sized([100.0, BUTTON], slider).changed() {
        asked = Some(Control::Zoom(value));
    }
    if icon_button(ui, Glyph::ZoomIn, labels.zoom_in).clicked() {
        asked = Some(Control::Zoom((zoom * ZOOM_STEP).min(max)));
    }
    let percent = format!("{:.0}%", zoom * 100.0);
    if ui.add(egui::Button::new(percent).small().frame(false)).on_hover_text(labels.zoom_reset).clicked() {
        asked = Some(Control::Zoom(1.0));
    }
    asked
}

/// `− 3 +`
fn depth_ui(ui: &mut egui::Ui, labels: &GraphLabels<'_>, depth: usize) -> Option<usize> {
    let mut asked = None;
    let less = ui.add_enabled(depth > *DEPTH.start(), egui::Button::new("−").small()).on_hover_text(labels.depth);
    if less.clicked() {
        asked = Some(depth - 1);
    }
    ui.label(depth.to_string()).on_hover_text(labels.depth);
    let more = ui.add_enabled(depth < *DEPTH.end(), egui::Button::new("+").small()).on_hover_text(labels.depth);
    if more.clicked() {
        asked = Some(depth + 1);
    }
    asked
}

/// What an icon button shows.
#[derive(Debug, Clone, Copy)]
enum Glyph {
    ZoomOut,
    ZoomIn,
    /// A target.
    Center,
    /// Four corners.
    Fit,
}

/// A small button drawn as `glyph`.
fn icon_button(ui: &mut egui::Ui, glyph: Glyph, hover: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(BUTTON), Sense::click());
    let visuals = ui.style().interact(&response);
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, visuals.bg_fill);
    }
    draw_glyph(ui.painter(), rect.shrink(4.0), glyph, Stroke::new(1.5, visuals.fg_stroke.color));
    response.on_hover_text(hover)
}

fn draw_glyph(painter: &egui::Painter, rect: Rect, glyph: Glyph, stroke: Stroke) {
    let c = rect.center();
    match glyph {
        Glyph::ZoomOut | Glyph::ZoomIn => {
            let lens = c - egui::vec2(1.5, 1.5);
            painter.circle_stroke(lens, 5.0, stroke);
            painter.line_segment([lens + egui::vec2(3.6, 3.6), lens + egui::vec2(7.0, 7.0)], stroke);
            painter.line_segment([lens - egui::vec2(2.5, 0.0), lens + egui::vec2(2.5, 0.0)], stroke);
            if matches!(glyph, Glyph::ZoomIn) {
                painter.line_segment([lens - egui::vec2(0.0, 2.5), lens + egui::vec2(0.0, 2.5)], stroke);
            }
        }
        Glyph::Center => {
            painter.circle_stroke(c, 4.5, stroke);
            painter.circle_filled(c, 1.2, stroke.color);
            for (from, to) in [
                ((0.0, -7.0), (0.0, -5.0)),
                ((0.0, 5.0), (0.0, 7.0)),
                ((-7.0, 0.0), (-5.0, 0.0)),
                ((5.0, 0.0), (7.0, 0.0)),
            ] {
                painter.line_segment([c + Vec2::from(from), c + Vec2::from(to)], stroke);
            }
        }
        Glyph::Fit => {
            for corner in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
                let inward = |to: Pos2| egui::vec2((to.x - corner.x).signum(), (to.y - corner.y).signum()) * 4.0;
                let toward = inward(c);
                painter.line_segment([corner, corner + egui::vec2(toward.x, 0.0)], stroke);
                painter.line_segment([corner, corner + egui::vec2(0.0, toward.y)], stroke);
            }
        }
    }
}

/// The zoom and pan that show `content` (on screen now, at `zoom` around `origin`) in `view`,
/// with a margin. `(zoom, pan)`.
pub(super) fn fit(
    view: Rect,
    content: Rect,
    (origin, zoom): (Pos2, f32),
    range: std::ops::RangeInclusive<f32>,
) -> (f32, Vec2) {
    let world =
        Rect::from_min_max(Pos2::ZERO + (content.min - origin) / zoom, Pos2::ZERO + (content.max - origin) / zoom);
    let scale = (view.width() / world.width().max(1.0)).min(view.height() / world.height().max(1.0)) * 0.92;
    let zoom = scale.clamp(*range.start(), *range.end());
    (zoom, -world.center().to_vec2() * zoom)
}

/// The pan at `to` zoom that keeps the middle of the view on the same place, from `pan` at `from`.
pub(super) fn zoom_pan(pan: Vec2, (from, to): (f32, f32)) -> Vec2 {
    pan * (to / from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitting_shows_all_the_nodes_in_the_middle() {
        let view = Rect::from_min_size(Pos2::ZERO, egui::vec2(1000.0, 500.0));
        // At zoom 1 around (500, 250): nodes from world (0, 0) to (2000, 500).
        let content = Rect::from_min_max(egui::pos2(500.0, 250.0), egui::pos2(2500.0, 750.0));
        let (zoom, pan) = fit(view, content, (view.center(), 1.0), 0.2..=2.5);
        assert!((zoom - 0.46).abs() < 0.001, "{zoom}");
        assert!((pan - egui::vec2(-1000.0, -250.0) * zoom).length() < 0.01, "{pan:?}");
    }

    #[test]
    fn zooming_keeps_the_middle() {
        assert_eq!(zoom_pan(egui::vec2(100.0, -50.0), (1.0, 2.0)), egui::vec2(200.0, -100.0));
    }
}
