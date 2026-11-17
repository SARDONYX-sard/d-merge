//! The scroll bars of the graph: where the view is among all the nodes, and a thumb to drag it.
//!
//! ```text
//! ┌──────────────────────────────┬─┐
//! │                              │█│  the view's part of the height of all the nodes (and the view)
//! │                              │ │
//! ├──────────────────────────────┘ │
//! │      ████                      │  and of their width
//! └────────────────────────────────┘
//! ```
//! A bar is shown only when the nodes go beyond the view that way.

use egui::{Rect, Sense, Vec2};

/// The bars' thickness.
const THICKNESS: f32 = 8.0;
/// A thumb is at least this long, to be grabbed.
const MIN_THUMB: f32 = 24.0;

/// Draws the bars over the bottom and the right of `view`. `content`: all the nodes, on screen.
/// Returns how far the nodes move on screen (a thumb dragged).
pub(super) fn scroll_bars(ui: &egui::Ui, painter: &egui::Painter, view: Rect, content: Rect) -> Vec2 {
    let all = content.union(view);
    Vec2::new(bar(ui, painter, (view, all), 0), bar(ui, painter, (view, all), 1))
}

/// The bar of `axis` (`0`: x, `1`: y). Returns how far the nodes move along it.
fn bar(ui: &egui::Ui, painter: &egui::Painter, (view, all): (Rect, Rect), axis: usize) -> f32 {
    let (view_len, all_len) = (view.size()[axis], all.size()[axis]);
    if all_len <= view_len + 1.0 {
        return 0.0;
    }

    let track = track(view, axis);
    let scale = track.size()[axis] / all_len;
    let start = (view.min[axis] - all.min[axis]).mul_add(scale, track.min[axis]);
    let len = (view_len * scale).max(MIN_THUMB);
    let mut thumb = track;
    thumb.min[axis] = start.min(track.max[axis] - len);
    thumb.max[axis] = thumb.min[axis] + len;

    let response = ui.interact(thumb, ui.scope_id().with(("graph_scroll", axis)), Sense::drag());
    let visuals = ui.visuals();
    let fill = if response.dragged() || response.hovered() {
        visuals.widgets.active.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    painter.rect_filled(track, THICKNESS / 2.0, visuals.extreme_bg_color.gamma_multiply(0.6));
    painter.rect_filled(thumb.shrink(1.0), THICKNESS / 2.0, fill);

    // The thumb goes with the pointer: the view as far, scaled; the nodes the other way.
    if response.dragged() { -response.drag_delta()[axis] / scale } else { 0.0 }
}

/// Along the bottom (`axis` 0) or the right of `view`, short of the corner.
fn track(view: Rect, axis: usize) -> Rect {
    if axis == 0 {
        Rect::from_min_max(
            egui::pos2(view.left(), view.bottom() - THICKNESS),
            egui::pos2(view.right() - THICKNESS, view.bottom()),
        )
    } else {
        Rect::from_min_max(
            egui::pos2(view.right() - THICKNESS, view.top()),
            egui::pos2(view.right(), view.bottom() - THICKNESS),
        )
    }
}
