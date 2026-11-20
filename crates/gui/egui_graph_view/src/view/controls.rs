//! The controls over the graph, for the mouse alone: the zoom (a slider between two magnifiers),
//! back to 100 %, the focus to the center, all the nodes in view, how deep the graph goes, and the
//! whole graph in place of the neighborhood.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │ [-] ━━━━━●━━━━ [+] [100%] [⌖] [⛶]  [−] 3 [+]  [≡]           │  at the bottom left, over the graph
//! └──────────────────────────────────────────────────────────────┘
//! ```

use egui::{Pos2, Rect, Vec2};
use egui_widgets::shadcn::{Button, Icon, Slider};

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
    /// Of the button that shows every node, not the neighborhood.
    pub all: &'a str,
    /// In the search of the nodes a pin dropped on no node can take.
    pub connect_hint: &'a str,
    /// In that list: the pin points nowhere then.
    pub disconnect: &'a str,
}

impl GraphLabels<'static> {
    pub const ENGLISH: Self = Self {
        zoom_out: "Zoom out",
        zoom_in: "Zoom in",
        zoom_reset: "Back to 100%",
        center: "Bring the focus to the center",
        fit: "Show all the nodes",
        depth: "How many layers right of the focus are shown",
        all: "Every node, not only the neighborhood of the focus",
        connect_hint: "Connect to…",
        disconnect: "Disconnect",
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
    /// Every node (`true`), or the neighborhood.
    All(bool),
}

/// A zoom button's step.
const ZOOM_STEP: f32 = 1.25;
/// The width of the zoom's slider.
const SLIDER_WIDTH: f32 = 100.0;
/// How deep the graph can go.
const DEPTH: std::ops::RangeInclusive<usize> = 1..=8;

/// Draws the controls at the bottom left of `view`. `(zoom, (depth, all))`: as they are now.
pub(super) fn controls_ui(
    ui: &mut egui::Ui,
    view: Rect,
    labels: &GraphLabels<'_>,
    ((zoom, zoom_range), (depth, all)): ((f32, std::ops::RangeInclusive<f32>), (usize, bool)),
) -> Option<Control> {
    let bar = Rect::from_min_size(view.left_bottom() + egui::vec2(8.0, -52.0), egui::vec2(490.0, 38.0));
    let builder = egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::left_to_right(egui::Align::Center));
    let mut asked = None;
    ui.scope_builder(builder, |ui| {
        let frame = egui::Frame::popup(ui.style()).inner_margin(egui::Margin::symmetric(6, 3));
        frame.show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            asked = zoom_ui(ui, labels, (zoom, zoom_range));
            ui.separator();
            if icon_button(ui, Icon::Crosshair, labels.center).clicked() {
                asked = Some(Control::Center);
            }
            if icon_button(ui, Icon::Maximize, labels.fit).clicked() {
                asked = Some(Control::Fit);
            }
            ui.separator();
            // The whole graph has no depth.
            if let Some(depth) = ui.add_enabled_ui(!all, |ui| depth_ui(ui, labels, depth)).inner {
                asked = Some(Control::Depth(depth));
            }
            let whole = ui.add(Button::selectable(all, "").icon(Icon::Layers).small()).on_hover_text(labels.all);
            if whole.clicked() {
                asked = Some(Control::All(!all));
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
    let zoom_out = icon_button(ui, Icon::ZoomOut, labels.zoom_out).clicked();
    let mut asked = zoom_out.then(|| Control::Zoom((zoom / ZOOM_STEP).max(min)));
    let mut value = zoom;
    if ui.add(Slider::new(&mut value, range).show_value(false).width(SLIDER_WIDTH)).changed() {
        asked = Some(Control::Zoom(value));
    }
    if icon_button(ui, Icon::ZoomIn, labels.zoom_in).clicked() {
        asked = Some(Control::Zoom((zoom * ZOOM_STEP).min(max)));
    }
    let percent = format!("{:.0}%", zoom * 100.0);
    if ui.add(Button::new(percent).small()).on_hover_text(labels.zoom_reset).clicked() {
        asked = Some(Control::Zoom(1.0));
    }
    asked
}

/// `− 3 +`
fn depth_ui(ui: &mut egui::Ui, labels: &GraphLabels<'_>, depth: usize) -> Option<usize> {
    let mut asked = None;
    let less = ui.add_enabled(depth > *DEPTH.start(), Button::new("−").small()).on_hover_text(labels.depth);
    if less.clicked() {
        asked = Some(depth - 1);
    }
    ui.label(depth.to_string()).on_hover_text(labels.depth);
    let more = ui.add_enabled(depth < *DEPTH.end(), Button::new("+").small()).on_hover_text(labels.depth);
    if more.clicked() {
        asked = Some(depth + 1);
    }
    asked
}

/// A small button of `icon` alone (shadcn), with `hover`.
fn icon_button(ui: &mut egui::Ui, icon: Icon, hover: &str) -> egui::Response {
    ui.add(Button::new("").icon(icon).small()).on_hover_text(hover)
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
