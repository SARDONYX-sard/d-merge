//! The graph on screen: its [`Layout`] with each node's size and place, pan and zoom, and what is
//! being done to it (a node moved, a value typed, a pin dragged, a page of a list turned).
//!
//! ```text
//! ┌──────────────────────────────────────────────┐  drag the background / wheel: pan
//! │ [Parent]──>[ Focus   ]──>[Child]  +3         │  Ctrl+wheel / pinch: zoom
//! │            [ name  x ]                       │  double click the background: center
//! │            [ ptr     ●]                      │  drag a title: move the node
//! └──────────────────────────────────────────────┘  drag a pin: connect it (see `wire`)
//! ```
//!
//! - [`rows`]: the rows kept of each node, and the pages of long lists.
//! - [`node`]: a node's card and lines, and what is done on them.
//! - [`wire`]: the edges (strong around the node under the pointer), and the pin being dragged.
//! - [`scroll`]: the scroll bars, over all the nodes.
//! - [`controls`]: zoom, center, fit and depth, for the mouse alone.

mod controls;
mod node;
mod rows;
mod scroll;
mod wire;

use std::cell::RefCell;

use egui::{Pos2, Rect, Sense, Vec2};
use fast_hash::HashMap;

pub use self::controls::GraphLabels;
use self::rows::{Line, NodeRows};
use crate::{
    GraphEdit, GraphSource,
    layout::{self, Layout, LayoutOptions},
};

/// What was done this frame.
#[derive(Debug)]
#[must_use]
pub struct GraphOutput<N> {
    /// A node's title.
    pub clicked: Option<N>,
    pub double_clicked: Option<N>,
    /// A row's name, or an edge's label: `(node, key)`, e.g. to show it in the text.
    pub row_clicked: Option<(N, usize)>,
    /// A value typed, a pin connected or disconnected: for the host to carry out.
    pub edit: Option<GraphEdit<N>>,
}

impl<N> Default for GraphOutput<N> {
    #[inline]
    fn default() -> Self {
        Self { clicked: None, double_clicked: None, row_clicked: None, edit: None }
    }
}

/// What the layout was computed for: another one computes it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayoutKey<N> {
    focus: N,
    version: u64,
    options: LayoutOptions,
    /// Changes when a page of a list does: other edges are followed.
    pages: u64,
}

/// The layout, with the rows kept of each node (the rows themselves are read from the source when
/// drawn).
#[derive(Debug)]
struct Placement<N> {
    key: LayoutKey<N>,
    layout: Layout<N>,
    rows: Vec<NodeRows>,
}

/// A value being typed: `(node, row key)` and the text so far.
#[derive(Debug)]
struct Editing<N> {
    node: N,
    key: usize,
    text: String,
    /// The field got the keyboard: it is asked for once.
    focused: bool,
}

/// A graph view's state: keep one per place it is shown.
#[derive(Debug)]
pub struct GraphView<N> {
    placement: Option<Placement<N>>,
    options: LayoutOptions,
    /// Where the world's origin is from the center of the view, in points.
    pan: Vec2,
    zoom: f32,
    /// Center the focus next frame.
    center: bool,
    /// Nodes the user dragged: how far from their place.
    moved: HashMap<N, Vec2>,
    editing: Option<Editing<N>>,
    /// The pin being dragged: `(node index, row key)`.
    wire: Option<(usize, usize)>,
    /// The page of each paged list: `(node, list)` -> page. None: the first.
    pages: HashMap<(N, usize), usize>,
    /// See [`LayoutKey::pages`].
    pages_version: u64,
    /// The highlight a page was turned to last: turned again only when it changes.
    followed: Option<usize>,
    /// The node under the pointer last frame: its edges are drawn strongly.
    hovered: Option<usize>,
    node_width: f32,
    gap: Vec2,
    list_page: usize,
    node_opacity: f32,
}

impl<N> Default for GraphView<N> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// The parts of a node, at zoom 1.
pub(super) mod metrics {
    /// The title and the subtitle.
    pub(super) const HEADER: f32 = 40.0;
    pub(super) const ROW: f32 = 20.0;
    /// Under the last row.
    pub(super) const BOTTOM: f32 = 6.0;

    /// The top of line `line`, from the node's top.
    #[expect(clippy::cast_precision_loss, reason = "a few rows")]
    pub(super) const fn line_top(line: usize) -> f32 {
        (line as f32).mul_add(ROW, HEADER)
    }

    /// `world` below `top` on screen, at `zoom`.
    pub(super) const fn below(top: f32, world: f32, zoom: f32) -> f32 {
        world.mul_add(zoom, top)
    }
}

impl<N> GraphView<N> {
    /// Default of [`Self::node_width`].
    pub const NODE_WIDTH: f32 = 230.0;
    /// Default of [`Self::gap`].
    pub const GAP: Vec2 = Vec2::new(90.0, 18.0);
    /// Default of [`Self::list_page`].
    pub const LIST_PAGE: usize = 8;
    /// Default of [`Self::node_opacity`].
    pub const NODE_OPACITY: f32 = 0.7;
    /// How far out and in it zooms.
    const ZOOM: std::ops::RangeInclusive<f32> = 0.2..=2.5;

    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            placement: None,
            options: LayoutOptions::DEFAULT,
            pan: Vec2::ZERO,
            zoom: 1.0,
            center: true,
            moved: HashMap::default(),
            editing: None,
            wire: None,
            pages: HashMap::default(),
            pages_version: 0,
            followed: None,
            hovered: None,
            node_width: Self::NODE_WIDTH,
            gap: Self::GAP,
            list_page: Self::LIST_PAGE,
            node_opacity: Self::NODE_OPACITY,
        }
    }

    /// How much of the neighborhood is shown.
    #[inline]
    #[must_use]
    pub const fn options(mut self, options: LayoutOptions) -> Self {
        self.options = options;
        self
    }

    /// The width of a node at zoom 1.
    #[inline]
    #[must_use]
    pub const fn node_width(mut self, node_width: f32) -> Self {
        self.node_width = node_width;
        self
    }

    /// The space between layers (`x`) and nodes (`y`) at zoom 1.
    #[inline]
    #[must_use]
    pub const fn gap(mut self, gap: Vec2) -> Self {
        self.gap = gap;
        self
    }

    /// The rows of a page of a list: a longer list is shown a page at a time, with `‹ ›`.
    #[inline]
    #[must_use]
    pub const fn list_page(mut self, list_page: usize) -> Self {
        self.list_page = list_page;
        self
    }

    /// How much a node's plate hides what is behind it, from `0` (glass) to `1`. Its text and pins
    /// are drawn fully.
    #[inline]
    #[must_use]
    pub const fn node_opacity(mut self, node_opacity: f32) -> Self {
        self.node_opacity = node_opacity;
        self
    }

    /// Changes how much is shown, e.g. from the host's settings.
    #[inline]
    pub const fn set_options(&mut self, options: LayoutOptions) {
        self.options = options;
    }

    #[inline]
    #[must_use]
    pub const fn current_options(&self) -> LayoutOptions {
        self.options
    }

    /// The focus goes to the center next frame, and moved nodes back to their places.
    #[inline]
    pub const fn recenter(&mut self) {
        self.center = true;
    }
}

impl<N: Copy + Eq + std::hash::Hash + std::fmt::Debug> GraphView<N> {
    /// Draws the neighborhood of `focus` in all the space left. `highlight`: the key of a row (and
    /// its edge) of the focus to draw strongly, e.g. of the line of the cursor in a text editor.
    /// `labels`: the hover texts of the controls ([`GraphLabels::ENGLISH`], or translated).
    pub fn show<S: GraphSource<Node = N>>(
        &mut self,
        ui: &mut egui::Ui,
        source: &S,
        (focus, highlight): (N, Option<usize>),
        labels: &GraphLabels<'_>,
    ) -> GraphOutput<N> {
        self.follow_highlight(focus, highlight);
        self.refresh(source, focus);
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.pan_and_zoom(ui, rect, &response);
        // What is drawn and clicked stays in the canvas: a node under a panel beside it (e.g. the
        // host's details) is not clicked through the panel.
        let mut canvas = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        canvas.set_clip_rect(rect.intersect(ui.clip_rect()));
        let ui = &mut canvas;

        let mut output = GraphOutput::default();
        let Some(placement) = self.placement.take() else { return output };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

        let frame = Frame { rect, origin: rect.center() + self.pan, zoom: self.zoom };
        let lines = self.lines(&placement);
        let rects = self.rects(&placement, &lines, frame);
        let pins = pins(&placement, &lines);
        let focus_index = placement.layout.index_of(focus);

        let geometry =
            wire::Geometry { frame, rects: &rects, pins: &pins, rows: &placement.rows, hovered: self.hovered };
        wire::edges(ui, &painter, (&placement.layout, &geometry), (focus_index, highlight), &mut output);
        let shown = node::Shown { placement: &placement, geometry: &geometry, lines: &lines };
        self.hovered = self.nodes_ui(ui, &painter, (source, &shown), (focus_index, highlight), &mut output);
        if let Some(edit) = self.wire_ui(ui, &painter, (source, &placement.layout, &geometry)) {
            output.edit = Some(edit);
        }
        wire::hidden_counts(&painter, (&placement.layout, &geometry), ui.visuals().weak_text_color());
        let content = rects.iter().copied().reduce(Rect::union);
        if let Some(content) = content {
            self.pan += scroll::scroll_bars(ui, &painter, rect, content);
        }
        let now = ((self.zoom, Self::ZOOM), self.options.depth_down);
        if let Some(control) = controls::controls_ui(ui, rect, labels, now) {
            self.apply_control(control, rect, content);
            ui.ctx().request_repaint();
        }

        if response.double_clicked() {
            self.center = true;
        }
        self.placement = Some(placement);
        output
    }

    /// Does what a control asked for. `content`: all the nodes on screen.
    fn apply_control(&mut self, control: controls::Control, view: Rect, content: Option<Rect>) {
        match control {
            controls::Control::Zoom(zoom) => {
                self.pan = controls::zoom_pan(self.pan, (self.zoom, zoom));
                self.zoom = zoom;
            }
            controls::Control::Center => self.center = true,
            controls::Control::Fit => {
                if let Some(content) = content {
                    let now = (view.center() + self.pan, self.zoom);
                    (self.zoom, self.pan) = controls::fit(view, content, now, Self::ZOOM);
                }
            }
            controls::Control::Depth(depth) => self.options.depth_down = depth,
        }
    }

    /// The page of `list` of `node`.
    fn page_of(&self, node: N, list: usize) -> usize {
        self.pages.get(&(node, list)).copied().unwrap_or_default()
    }

    /// Turns `list` of `node` to `page`: the layout follows other edges then.
    fn turn_page(&mut self, node: N, (list, page): (usize, usize)) {
        if self.page_of(node, list) != page {
            self.pages.insert((node, list), page);
            self.pages_version += 1;
        }
    }

    /// A new highlight on a paged list of the focus: its page is turned to it.
    fn follow_highlight(&mut self, focus: N, highlight: Option<usize>) {
        if std::mem::replace(&mut self.followed, highlight) == highlight {
            return;
        }
        let Some(key) = highlight else { return };
        let Some(placement) = &self.placement else { return };
        let Some(index) = placement.layout.index_of(focus) else { return };
        if let Some(page) = placement.rows[index].page_of_key(key, self.list_page) {
            self.turn_page(focus, page);
        }
    }

    /// Computes the layout again if the focus, the source or a page changed. A new focus is
    /// centered. The rows of nodes are read again only when the source changed.
    fn refresh<S: GraphSource<Node = N>>(&mut self, source: &S, focus: N) {
        let key = LayoutKey { focus, version: source.version(), options: self.options, pages: self.pages_version };
        let old = self.placement.take();
        if old.as_ref().is_some_and(|old| old.key == key) {
            self.placement = old;
            return;
        }
        if old.as_ref().is_none_or(|old| old.key.focus != focus) {
            self.center = true;
        }

        // Rows read for the layout are kept for the drawing; of the same source, those of before.
        let mut kept: HashMap<N, NodeRows> = HashMap::default();
        if let Some(old) = old.filter(|old| old.key.version == key.version) {
            kept.extend(old.layout.nodes.iter().map(|placed| placed.node).zip(old.rows));
        }
        let cache = RefCell::new(kept);
        let per_page = self.list_page;
        let keep = |node: N, key: usize| {
            let mut cache = cache.borrow_mut();
            let rows = cache.entry(node).or_insert_with(|| NodeRows::read(source.rows(node)));
            rows.is_shown(key, |list| self.page_of(node, list), per_page)
        };
        let layout = layout::layout(source, focus, self.options, &keep);

        let mut cache = cache.into_inner();
        let rows = layout
            .nodes
            .iter()
            .map(|placed| cache.remove(&placed.node).unwrap_or_else(|| NodeRows::read(source.rows(placed.node))))
            .collect();
        self.placement = Some(Placement { key, layout, rows });
    }

    /// The lines of each node, with the pages turned now.
    fn lines(&self, placement: &Placement<N>) -> Vec<Vec<Line>> {
        let nodes = placement.layout.nodes.iter().zip(&placement.rows);
        nodes.map(|(placed, rows)| rows.lines(|list| self.page_of(placed.node, list), self.list_page)).collect()
    }

    /// Each node on screen: the layers stacked by the nodes' heights, then moved by the user.
    fn rects(&self, placement: &Placement<N>, lines: &[Vec<Line>], frame: Frame) -> Vec<Rect> {
        let heights: Vec<f32> = lines.iter().map(|lines| node_height(lines.len())).collect();
        let centers = placement.layout.stack(&heights, self.gap.y);
        let nodes = placement.layout.nodes.iter().enumerate();
        nodes
            .map(|(i, placed)| {
                #[expect(clippy::cast_precision_loss, reason = "a few layers")]
                let x = placed.layer as f32 * (self.node_width + self.gap.x);
                let moved = self.moved.get(&placed.node).copied().unwrap_or_default();
                let size = egui::vec2(self.node_width, heights[i]);
                frame.screen(Rect::from_center_size(egui::pos2(x, centers[i]) + moved, size))
            })
            .collect()
    }

    /// Drag and wheel pan, `Ctrl+wheel` and pinch zoom around the pointer.
    fn pan_and_zoom(&mut self, ui: &egui::Ui, rect: Rect, response: &egui::Response) {
        if std::mem::take(&mut self.center) {
            self.pan = Vec2::ZERO; // the focus is at the world's origin
            self.moved.clear();
        }
        if response.dragged() {
            self.pan += response.drag_delta();
        }
        if !response.contains_pointer() {
            return;
        }

        let (scroll, zoom_delta, pointer) =
            ui.input(|input| (input.smooth_scroll_delta, input.zoom_delta(), input.pointer.hover_pos()));
        self.pan += scroll;
        if (zoom_delta - 1.0).abs() > f32::EPSILON
            && let Some(pointer) = pointer
        {
            let zoom = (self.zoom * zoom_delta).clamp(*Self::ZOOM.start(), *Self::ZOOM.end());
            let origin = rect.center() + self.pan;
            let world = (pointer - origin) / self.zoom;
            self.pan = pointer - world * zoom - rect.center();
            self.zoom = zoom;
        }
    }

    /// The nodes on screen, and what was done on them. Returns the node under the pointer.
    fn nodes_ui<S: GraphSource<Node = N>>(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        (source, shown): (&S, &node::Shown<'_, N>),
        (focus, highlight): (Option<usize>, Option<usize>),
        output: &mut GraphOutput<N>,
    ) -> Option<usize> {
        let layout = &shown.placement.layout;
        let wire_from = self.wire.map(|(index, key)| (layout.nodes[index].node, key));
        let mut hovered = None;
        for (i, placed) in layout.nodes.iter().enumerate() {
            let rect = shown.geometry.rects[i];
            if !shown.geometry.frame.rect.intersects(rect) {
                continue;
            }
            if ui.rect_contains_pointer(rect) {
                hovered = Some(i);
            }

            let is_focus = Some(i) == focus;
            let rows = &shown.placement.rows[i];
            let pages: Vec<usize> = (0..rows.lists.len()).map(|list| self.page_of(placed.node, list)).collect();
            let input = node::NodeInput {
                source,
                node: placed.node,
                index: i,
                rect,
                zoom: shown.geometry.frame.zoom,
                opacity: self.node_opacity,
                lines: (&shown.lines[i], rows, &pages, self.list_page),
                is_focus,
                highlight: highlight.filter(|_| is_focus),
                wire_accepts: wire_from
                    .map(|(from, key)| from != placed.node && source.accepts(from, key, placed.node)),
            };
            let state = node::NodeState { editing: &mut self.editing, wire: &mut self.wire };
            let done = node::node_ui(ui, painter, input, state);
            self.take_node_done(placed.node, done, output);
        }
        hovered
    }

    /// What was done on `node`, into the view and `output`.
    fn take_node_done(&mut self, node: N, done: node::NodeDone<N>, output: &mut GraphOutput<N>) {
        if let Some(delta) = done.moved {
            *self.moved.entry(node).or_default() += delta / self.zoom;
        }
        if let Some(page) = done.page {
            self.turn_page(node, page);
        }
        if done.clicked {
            output.clicked = Some(node);
        }
        if done.double_clicked {
            output.double_clicked = Some(node);
        }
        if let Some(key) = done.row_clicked {
            output.row_clicked = Some((node, key));
        }
        if let Some(edit) = done.edit {
            output.edit = Some(edit);
        }
    }

    /// The pin being dragged: its wire, and what dropping it does.
    fn wire_ui<S: GraphSource<Node = N>>(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        (source, layout, geometry): (&S, &Layout<N>, &wire::Geometry<'_>),
    ) -> Option<GraphEdit<N>> {
        let (index, key) = self.wire?;
        let pointer = ui.input(|input| input.pointer.interact_pos())?;
        let color = geometry.edge_color(index, key).unwrap_or_else(|| ui.visuals().selection.stroke.color);
        wire::draw_wire(painter, (geometry.pin(index, key), pointer), geometry.frame.zoom, color);
        ui.ctx().request_repaint();

        if ui.input(|input| input.pointer.primary_down()) {
            return None;
        }
        self.wire = None;
        let from = layout.nodes[index].node;
        let target = geometry.node_at(pointer).map(|to| layout.nodes[to].node).filter(|&to| to != from);
        match target {
            Some(to) if source.accepts(from, key, to) => Some(GraphEdit::Connect { node: from, key, to }),
            Some(_) => None,
            None => {
                let connected = crate::Slot::Pin { connected: true };
                let is_connected = source.rows(from).any(|row| row.key == key && row.slot == connected);
                is_connected.then_some(GraphEdit::Disconnect { node: from, key })
            }
        }
    }
}

/// The line of each shown row: `(node index, key)` -> line.
fn pins<N>(placement: &Placement<N>, lines: &[Vec<Line>]) -> HashMap<(usize, usize), usize> {
    let mut pins = HashMap::default();
    for (i, node_lines) in lines.iter().enumerate() {
        for (at, line) in node_lines.iter().enumerate() {
            if let Line::Row(row) = *line {
                pins.insert((i, placement.rows[i].rows[row].key), at);
            }
        }
    }
    pins
}

/// A node with `lines` lines, at zoom 1.
const fn node_height(lines: usize) -> f32 {
    if lines == 0 { metrics::HEADER } else { metrics::line_top(lines) + metrics::BOTTOM }
}

/// From the world (zoom 1, focus at the origin) to the screen.
#[derive(Debug, Clone, Copy)]
pub(super) struct Frame {
    pub(super) rect: Rect,
    pub(super) origin: Pos2,
    pub(super) zoom: f32,
}

impl Frame {
    fn screen(self, world: Rect) -> Rect {
        Rect::from_min_max(self.origin + world.min.to_vec2() * self.zoom, self.origin + world.max.to_vec2() * self.zoom)
    }
}
