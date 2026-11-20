//! The edges, and the wire of a pin being dragged: a curve from a pin on the right of one node to
//! the port on the left of another, in the pin's color.
//!
//! ```text
//! ┌──────────┐            ┌────────┐
//! │ Focus    │        ┌─>○│ Child  │   ● a pin (a row's), ○ the port (the title's)
//! │ states  ●─────────┘   └────────┘
//! └──────────┘
//! ```
//!
//! A big graph's edges are a web. So that its shape can be read and an edge followed:
//!
//! ```text
//! edge                          drawn
//! of the tree (see `layout`)    solid, in its color
//! to a node met already         dashed and faint: a shared node's other parents
//! of the node under the pointer solid and thick; then all the others faint
//! of the cursor's row           thickest, in the selection's color
//! ```

use egui::{Color32, Pos2, Rect, Sense, Stroke, epaint::CubicBezierShape};
use fast_hash::HashMap;

use super::{Frame, GraphOutput, metrics, rows::NodeRows};
use crate::{layout::Layout, tree::Tree};

/// Below this zoom, no edge labels: they could not be read.
const LABEL_ZOOM: f32 = 0.6;

/// Where the nodes are on screen this frame.
#[derive(Debug)]
pub(super) struct Geometry<'a> {
    pub(super) frame: Frame,
    pub(super) rects: &'a [Rect],
    /// The line of each shown row: `(node index, key)` -> line.
    pub(super) pins: &'a HashMap<(usize, usize), usize>,
    pub(super) rows: &'a [NodeRows],
    /// The node under the pointer.
    pub(super) hovered: Option<usize>,
    /// The whole graph's: its tree, and the room between two columns on screen. See [`whole_route`].
    pub(super) whole: Option<(&'a Tree, f32)>,
}

impl Geometry<'_> {
    /// The pin of row `key` of node `index`: the title's right if no line shows it.
    pub(super) fn pin(&self, index: usize, key: usize) -> Pos2 {
        let rect = self.rects[index];
        let y = match self.pins.get(&(index, key)) {
            Some(&line) => metrics::line_top(line) + metrics::ROW / 2.0,
            None => metrics::HEADER / 2.0,
        };
        egui::pos2(rect.right(), metrics::below(rect.top(), y, self.frame.zoom))
    }

    /// Where edges come into node `index`.
    pub(super) fn port(&self, index: usize) -> Pos2 {
        let rect = self.rects[index];
        egui::pos2(rect.left(), metrics::below(rect.top(), metrics::HEADER / 2.0, self.frame.zoom))
    }

    /// The node on screen under `at`.
    pub(super) fn node_at(&self, at: Pos2) -> Option<usize> {
        if !self.frame.rect.contains(at) {
            return None;
        }
        self.rects.iter().rposition(|rect| rect.contains(at))
    }

    /// The color of the row of `key` of node `index`.
    pub(super) fn edge_color(&self, index: usize, key: usize) -> Option<Color32> {
        let rows = &self.rows[index];
        rows.row_of(key).and_then(|row| rows.rows[row].color)
    }

    /// Is the row of `key` of node `index` on a page not shown?
    fn is_paged_out(&self, index: usize, key: usize) -> bool {
        self.rows[index].row_of(key).is_some() && !self.pins.contains_key(&(index, key))
    }
}

/// The edges, behind the nodes. One whose row is not shown has its label drawn on it.
pub(super) fn edges<N: Copy + Eq + std::hash::Hash>(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (layout, geometry): (&Layout<N>, &Geometry<'_>),
    (focus, highlight): (Option<usize>, Option<usize>),
    output: &mut GraphOutput<N>,
) {
    let visuals = ui.visuals();
    let zoom = geometry.frame.zoom;
    for (i, edge) in layout.edges.iter().enumerate() {
        // The whole graph's cards show no rows: none is paged out.
        if geometry.whole.is_none() && edge.to != 0 && geometry.is_paged_out(edge.from, edge.key) {
            continue;
        }
        let (start, end) = (geometry.pin(edge.from, edge.key), geometry.port(edge.to));
        if let Some((tree, room)) = geometry.whole {
            let of_tree = tree.parent(edge.to) == Some(edge.from);
            let lane_of = if of_tree { None } else { Some(edge.to) };
            let points = whole_route((start, end), room, lane_of);
            whole_edge(ui, painter, (geometry, &points), (edge.from, edge.to, edge.key), of_tree);
            continue;
        }
        let points = curve(start, end, zoom);
        if !geometry.frame.rect.intersects(Rect::from_points(&points)) {
            continue;
        }

        let strong = Some(edge.from) == focus && highlight == Some(edge.key);
        let touched = geometry.hovered.map(|hovered| hovered == edge.from || hovered == edge.to);
        let color = geometry.edge_color(edge.from, edge.key).unwrap_or_else(|| visuals.weak_text_color());
        let look =
            if strong { Look::Cursor(visuals.selection.stroke.color) } else { Look::of(edge.tree, touched, color) };
        let (stroke, dashed) = look.stroke(zoom);
        let shape = CubicBezierShape::from_points_stroke(points, false, Color32::TRANSPARENT, stroke);
        let middle = shape.sample(0.5);
        if dashed {
            let points = shape.flatten(Some(0.5));
            painter.extend(egui::Shape::dashed_line(&points, stroke, 6.0 * zoom, 4.0 * zoom));
        } else {
            painter.add(shape);
        }
        arrow_head(painter, points[3], zoom, stroke.color);

        let has_line = geometry.pins.contains_key(&(edge.from, edge.key));
        if !has_line && zoom >= LABEL_ZOOM && !edge.label.is_empty() {
            let id = ui.scope_id().with(("graph_edge", i));
            if edge_label(ui, painter, (id, middle, zoom), (&edge.label, strong)).clicked() {
                output.row_clicked = Some((layout.nodes[edge.from].node, edge.key));
            }
        }
    }
}

/// The square way of an edge of the whole graph from `start` (a pin) to `end` (a port), `room`
/// apart from one column to the next (on screen):
///
/// ```text
///  [Parent]●─┬──────>[Child]     of the tree: down its parent's trunk, in the middle of the room,
///            └──────>[Child]     as the branches of a tree
///  [Other]●──────┐
///                └──>[Shared]    else down a lane of its own, nearer the node it goes to
/// ```
///
/// A lane is never a trunk: an edge on one would seem joined to the branches it overlaps. The lanes
/// of the nodes (`lane_of`) are a few, side by side, so that two of them rarely overlap either. An
/// edge to the left (a cycle) goes straight.
fn whole_route((start, end): (Pos2, Pos2), room: f32, lane_of: Option<usize>) -> Vec<Pos2> {
    /// Where the lanes are, back from the node they go to, as a share of the room: right of the
    /// trunks, in its middle.
    const LANES: [f32; 3] = [0.15, 0.25, 0.35];
    let x = match lane_of {
        None => room.mul_add(0.5, start.x),
        Some(node) => room.mul_add(-LANES[node % LANES.len()], end.x),
    };
    if x <= start.x {
        return vec![start, end];
    }
    vec![start, egui::pos2(x, start.y), egui::pos2(x, end.y), end]
}

/// An edge of the whole graph through `points` (see [`whole_route`]). Not of the tree (`tree`):
/// fainter, but solid: none looks broken off.
fn whole_edge(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (geometry, points): (&Geometry<'_>, &[Pos2]),
    (from, to, key): (usize, usize, usize),
    tree: bool,
) {
    if !geometry.frame.rect.intersects(Rect::from_points(points)) {
        return;
    }
    let touched = geometry.hovered.map(|hovered| hovered == from || hovered == to);
    let color = geometry.edge_color(from, key).unwrap_or_else(|| ui.visuals().weak_text_color());
    let (stroke, _) = Look::of(tree, touched, color).stroke(geometry.frame.zoom);
    painter.add(egui::Shape::line(points.to_vec(), stroke));
    if let [.., before, tip] = points {
        arrow_along(painter, (*before, *tip), geometry.frame.zoom, stroke.color);
    }
}

/// An arrow head at `tip`, pointing the way from `from`.
fn arrow_along(painter: &egui::Painter, (from, tip): (Pos2, Pos2), zoom: f32, color: Color32) {
    let size = 6.0 * zoom.max(0.5);
    let back = (from - tip).normalized() * size;
    let side = egui::vec2(-back.y, back.x) / 2.0;
    let points = vec![tip, tip + back + side, tip + back - side];
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

/// How an edge is drawn: see the module docs.
#[derive(Debug, Clone, Copy)]
enum Look {
    Cursor(Color32),
    /// Of the node under the pointer.
    Pointed(Color32),
    /// Not of the node under the pointer.
    Behind(Color32),
    Tree(Color32),
    /// To a node met already.
    Cross(Color32),
}

impl Look {
    /// `touched`: whether the edge is the pointed node's, if a node is pointed at.
    const fn of(tree: bool, touched: Option<bool>, color: Color32) -> Self {
        match (touched, tree) {
            (Some(true), _) => Self::Pointed(color),
            (Some(false), _) => Self::Behind(color),
            (None, true) => Self::Tree(color),
            (None, false) => Self::Cross(color),
        }
    }

    /// `(stroke, dashed)` at `zoom`.
    fn stroke(self, zoom: f32) -> (Stroke, bool) {
        let width = |width: f32| width * zoom.max(0.5);
        match self {
            Self::Cursor(color) => (Stroke::new(width(3.0), color), false),
            Self::Pointed(color) => (Stroke::new(width(2.5), color), false),
            Self::Behind(color) => (Stroke::new(width(1.0), color.gamma_multiply(0.12)), false),
            Self::Tree(color) => (Stroke::new(width(1.6), color.gamma_multiply(0.85)), false),
            Self::Cross(color) => (Stroke::new(width(1.0), color.gamma_multiply(0.35)), true),
        }
    }
}

/// The wire of a pin being dragged, from it to the pointer.
pub(super) fn draw_wire(painter: &egui::Painter, (start, end): (Pos2, Pos2), zoom: f32, color: Color32) {
    let stroke = Stroke::new(2.5 * zoom.max(0.5), color);
    painter.add(CubicBezierShape::from_points_stroke(curve(start, end, zoom), false, Color32::TRANSPARENT, stroke));
}

/// `[start, control, control, end]`: leaves to the right, comes in from the left, also backwards.
fn curve(start: Pos2, end: Pos2, zoom: f32) -> [Pos2; 4] {
    let bend = ((end.x - start.x).abs() / 2.0).max(40.0 * zoom);
    [start, start + egui::vec2(bend, 0.0), end - egui::vec2(bend, 0.0), end]
}

fn arrow_head(painter: &egui::Painter, tip: Pos2, zoom: f32, color: Color32) {
    let size = 6.0 * zoom.max(0.5);
    let points = vec![tip, tip + egui::vec2(-size, -size / 2.0), tip + egui::vec2(-size, size / 2.0)];
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}

/// An edge's label on a plate: a click shows the edge (e.g. its line in the text).
fn edge_label(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (id, at, zoom): (egui::Id, Pos2, f32),
    (label, strong): (&str, bool),
) -> egui::Response {
    let visuals = ui.visuals();
    let color = if strong { visuals.strong_text_color() } else { visuals.text_color() };
    let galley = painter.layout_no_wrap(label.to_owned(), egui::FontId::proportional(11.0 * zoom), color);
    let rect = Rect::from_center_size(at, galley.size() + egui::vec2(8.0, 2.0) * zoom);
    let response = ui.interact(rect, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);

    let fill = if response.hovered() { visuals.widgets.hovered.bg_fill } else { visuals.panel_fill };
    painter.rect_filled(rect, 3.0 * zoom, fill);
    painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

/// `+3` under a layer whose other nodes did not fit.
pub(super) fn hidden_counts<N: Copy + Eq + std::hash::Hash>(
    painter: &egui::Painter,
    (layout, geometry): (&Layout<N>, &Geometry<'_>),
    color: Color32,
) {
    for layer in layout.layers.iter().filter(|layer| layer.hidden > 0) {
        let last = layout.nodes.iter().position(|placed| placed.layer == layer.layer && placed.row + 1 == layer.len);
        let Some(last) = last else { continue };
        let zoom = geometry.frame.zoom;
        let below = geometry.rects[last].center_bottom() + egui::vec2(0.0, 6.0 * zoom);
        let font = egui::FontId::proportional(12.0 * zoom);
        painter.text(below, egui::Align2::CENTER_TOP, format!("+{}", layer.hidden), font, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree edge goes down the middle of the room (the trunk); another down a lane nearer the
    /// node it goes to, never on the trunk; one to the left, straight.
    #[test]
    fn whole_edges_keep_off_the_trunks() {
        let (start, end) = (egui::pos2(0.0, 0.0), egui::pos2(100.0, 50.0));
        let x_of = |route: &[Pos2]| route[1].x;
        assert!((x_of(&whole_route((start, end), 100.0, None)) - 50.0).abs() < f32::EPSILON);
        for node in 0..3 {
            let lane = x_of(&whole_route((start, end), 100.0, Some(node)));
            assert!((60.0..90.0).contains(&lane), "between the trunk and the node: {lane}");
        }
        assert_eq!(whole_route((end, start), 100.0, Some(0)).len(), 2, "to the left: straight");
    }
}
