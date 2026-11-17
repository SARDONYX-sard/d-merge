//! A node: its card (title, subtitle, header band, `×N` if shared) over its lines, and what is done
//! on them.
//!
//! ```text
//! ○┌▓Root▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ×2┐   title: click, double click, drag to move it
//!  │ #0095 hkbStateMachine │
//!  │ startStateId       0  │   a value: click to type (Enter: done, Esc: cancel)
//!  │ states  9-16 / 900 ‹ ›│   a long list's pager: a page of it below
//!  │ states[8]          ●  │   a pin: drag onto a node to connect, off to disconnect
//!  │ states             +● │   the end of a list: drop a node to add it
//!  └───────────────────────┘   a row's name: click to show it (e.g. in the text)
//! ```

use egui::{Color32, Rect, Sense, Stroke, Vec2};

use super::{
    Editing, Placement, metrics,
    rows::{Line, NodeRows},
    wire::Geometry,
};
use crate::{Card, GraphEdit, GraphSource, Row, Slot};

/// Below this zoom, no text: it could not be read, nor typed in.
const TEXT_ZOOM: f32 = 0.4;
/// A pin's radius at zoom 1.
const PIN: f32 = 4.5;

/// What is shown this frame, for all the nodes.
pub(super) struct Shown<'a, N> {
    pub(super) placement: &'a Placement<N>,
    pub(super) geometry: &'a Geometry<'a>,
    pub(super) lines: &'a [Vec<Line>],
}

/// What a node is drawn with.
pub(super) struct NodeInput<'a, S: GraphSource> {
    pub(super) source: &'a S,
    pub(super) node: S::Node,
    /// In the layout: for the ids of its parts.
    pub(super) index: usize,
    pub(super) rect: Rect,
    pub(super) zoom: f32,
    /// Of its plate: see `GraphView::node_opacity`.
    pub(super) opacity: f32,
    /// `(its lines, its rows, the page of each of its lists, rows of a page)`.
    pub(super) lines: (&'a [Line], &'a NodeRows, &'a [usize], usize),
    pub(super) is_focus: bool,
    /// The key of the row to draw strongly.
    pub(super) highlight: Option<usize>,
    /// A pin is being dragged: can this node take it?
    pub(super) wire_accepts: Option<bool>,
}

/// What a node changes of the view.
pub(super) struct NodeState<'a, N> {
    pub(super) editing: &'a mut Option<Editing<N>>,
    pub(super) wire: &'a mut Option<(usize, usize)>,
}

/// What was done on a node this frame.
pub(super) struct NodeDone<N> {
    pub(super) clicked: bool,
    pub(super) double_clicked: bool,
    pub(super) row_clicked: Option<usize>,
    /// Its title was dragged this far, on screen.
    pub(super) moved: Option<Vec2>,
    /// `(list, page)`: a pager was clicked.
    pub(super) page: Option<(usize, usize)>,
    pub(super) edit: Option<GraphEdit<N>>,
}

pub(super) fn node_ui<S: GraphSource>(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    input: NodeInput<'_, S>,
    mut state: NodeState<'_, S::Node>,
) -> NodeDone<S::Node> {
    let card = input.source.card(input.node);
    let header = Rect::from_min_size(input.rect.min, egui::vec2(input.rect.width(), metrics::HEADER * input.zoom));
    plate(ui, painter, (&input, header), &card);

    let id = ui.scope_id().with(("graph_node", input.index));
    let hover = format!("{}\n{}", card.title, card.subtitle);
    let response = ui.interact(header, id, Sense::click_and_drag()).on_hover_text(hover);
    let mut done = NodeDone {
        clicked: response.clicked(),
        double_clicked: response.double_clicked(),
        row_clicked: None,
        moved: response.dragged().then(|| response.drag_delta()),
        page: None,
        edit: None,
    };
    if input.zoom < TEXT_ZOOM {
        return done;
    }

    title(ui, painter, (header, input.zoom), &card);
    let rows: Vec<Row<'_>> = input.source.rows(input.node).collect();
    for (at, line) in input.lines.0.iter().enumerate() {
        let top = metrics::below(input.rect.top(), metrics::line_top(at), input.zoom);
        let size = egui::vec2(input.rect.width(), metrics::ROW * input.zoom);
        let rect = Rect::from_min_size(egui::pos2(input.rect.left(), top), size);
        if !painter.clip_rect().intersects(rect) {
            continue;
        }
        match *line {
            Line::Row(row) => {
                if let Some(&row) = rows.get(row) {
                    row_ui(ui, painter, (&input, rect, card.editable), row, &mut state, &mut done);
                }
            }
            Line::Pager(list) => pager_ui(ui, painter, (&input, rect), (list, &rows), &mut done),
        }
    }
    done
}

/// The node's plate, its header band and accent, `×N` if shared, and its port on the left.
fn plate<S: GraphSource>(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (input, header): (&NodeInput<'_, S>, Rect),
    card: &Card<'_>,
) {
    let visuals = ui.visuals();
    let faint = card.dimmed || input.wire_accepts == Some(false);
    let tone = |color: Color32| if faint { color.gamma_multiply(0.4) } else { color };
    let zoom = input.zoom;
    let rect = input.rect;
    let radius = (6.0 * zoom).clamp(0.0, 255.0);

    let stroke = if input.wire_accepts == Some(true) || input.is_focus {
        Stroke::new(2.0, visuals.selection.stroke.color)
    } else {
        Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color)
    };
    painter.rect_filled(rect, radius, tone(visuals.window_fill.gamma_multiply(input.opacity)));
    if let Some(color) = card.header {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to a byte")]
        let corner = radius as u8;
        let rounding = egui::CornerRadius { nw: corner, ne: corner, ..Default::default() };
        painter.rect_filled(header, rounding, tone(color.gamma_multiply(0.55)));
    }
    if let Some(accent) = card.accent {
        let bar = Rect::from_min_size(rect.min, egui::vec2(4.0 * zoom, rect.height()));
        painter.rect_filled(bar, egui::CornerRadius { nw: 6, sw: 6, ..Default::default() }, tone(accent));
    }
    painter.rect_stroke(rect, radius, stroke, egui::StrokeKind::Inside);

    if card.fan_in > 1 && zoom >= TEXT_ZOOM {
        let font = egui::FontId::proportional(10.5 * zoom);
        let at = header.right_top() + egui::vec2(-8.0, 5.0) * zoom;
        let text = format!("×{}", card.fan_in);
        painter.text(at, egui::Align2::RIGHT_TOP, text, font, tone(visuals.strong_text_color()));
    }

    let port = egui::pos2(rect.left(), header.center().y);
    painter.circle(port, PIN * zoom, visuals.panel_fill, Stroke::new(1.5, tone(visuals.text_color())));
}

/// The title over the subtitle.
fn title(ui: &egui::Ui, painter: &egui::Painter, (header, zoom): (Rect, f32), card: &Card<'_>) {
    let visuals = ui.visuals();
    let tone = |color: Color32| if card.dimmed { color.gamma_multiply(0.5) } else { color };
    let inner = header.shrink2(egui::vec2(12.0, 5.0) * zoom);
    let badge = if card.fan_in > 1 { 30.0 * zoom } else { 0.0 };
    let width = inner.width() - badge;
    let title = one_row(painter, (card.title, 13.0 * zoom), tone(visuals.strong_text_color()), width);
    let subtitle = one_row(painter, (card.subtitle, 10.5 * zoom), tone(visuals.text_color()), inner.width());
    painter.galley(inner.left_top(), title, Color32::PLACEHOLDER);
    painter.galley(inner.left_bottom() - egui::vec2(0.0, subtitle.size().y), subtitle, Color32::PLACEHOLDER);
}

/// One row: its name on the left, its value or pin on the right.
fn row_ui<S: GraphSource>(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    (input, rect, editable): (&NodeInput<'_, S>, Rect, bool),
    row: Row<'_>,
    state: &mut NodeState<'_, S::Node>,
    done: &mut NodeDone<S::Node>,
) {
    let visuals = ui.visuals().clone();
    let zoom = input.zoom;
    if input.highlight == Some(row.key) {
        painter.rect_filled(rect, 0.0, visuals.selection.bg_fill.gamma_multiply(0.5));
    }

    let inner = rect.shrink2(egui::vec2(12.0, 0.0) * zoom);
    let (name_rect, value_rect) = inner.split_left_right_at_fraction(0.5);
    let id = ui.scope_id().with(("graph_row", input.index, row.key));
    let name = ui.interact(name_rect, id.with("name"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if name.clicked() {
        done.row_clicked = Some(row.key);
    }
    let label = match row.index {
        Some(index) => std::borrow::Cow::Owned(format!("{}[{index}]", row.label)),
        None => std::borrow::Cow::Borrowed(row.label),
    };
    let galley = one_row(painter, (&label, 11.0 * zoom), visuals.text_color(), name_rect.width());
    painter.galley(egui::pos2(name_rect.left(), rect.center().y - galley.size().y / 2.0), galley, Color32::PLACEHOLDER);

    let color = row.color.unwrap_or_else(|| visuals.text_color());
    let pin = egui::pos2(rect.right(), rect.center().y);
    match row.slot {
        Slot::Value(text) => {
            value_ui(ui, painter, (input, value_rect, editable, id), (row.key, text, color), state, done);
        }
        Slot::Fixed(text) => right_text(painter, value_rect, (text, 11.0 * zoom), visuals.weak_text_color()),
        Slot::Pin { connected } => pin_ui(ui, painter, (input, pin, editable, id), (row.key, connected, color), state),
        Slot::AddPin => {
            right_text(painter, value_rect, ("+", 13.0 * zoom), visuals.weak_text_color());
            pin_ui(ui, painter, (input, pin, editable, id), (row.key, false, color), state);
        }
    }
}

/// A long list's pager: `states  9-16 / 900  ‹ ›`.
fn pager_ui<S: GraphSource>(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (input, rect): (&NodeInput<'_, S>, Rect),
    (list, rows): (usize, &[Row<'_>]),
    done: &mut NodeDone<S::Node>,
) {
    let (_, node_rows, pages, per_page) = input.lines;
    let shown = node_rows.page(list, pages[list], per_page);
    let start = node_rows.lists[list].start;
    let len = node_rows.lists[list].len;
    let page = (shown.start - start) / per_page.max(1);
    let zoom = input.zoom;
    let visuals = ui.visuals();

    painter.rect_filled(rect, 0.0, visuals.faint_bg_color);
    let label = rows.get(start).map_or("", |row| row.label);
    let text = format!("{label}  {}-{} / {len}", shown.start - start + 1, shown.end - start);
    let inner = rect.shrink2(egui::vec2(12.0, 0.0) * zoom);
    let galley =
        one_row(painter, (&text, 11.0 * zoom), visuals.weak_text_color(), 40.0_f32.mul_add(-zoom, inner.width()));
    painter.galley(egui::pos2(inner.left(), rect.center().y - galley.size().y / 2.0), galley, Color32::PLACEHOLDER);

    let button = egui::vec2(16.0, rect.height() / zoom) * zoom;
    let next = Rect::from_min_size(egui::pos2(inner.right() - button.x, rect.top()), button);
    let previous = next.translate(egui::vec2(4.0_f32.mul_add(-zoom, -button.x), 0.0));
    let id = ui.scope_id().with(("graph_pager", input.index, list));
    let buttons = [(previous, "‹", page > 0, page.saturating_sub(1)), (next, "›", shown.end < start + len, page + 1)];
    for (at, (button_rect, glyph, enabled, to)) in buttons.into_iter().enumerate() {
        let color = if enabled { visuals.strong_text_color() } else { visuals.weak_text_color().gamma_multiply(0.5) };
        right_text(painter, button_rect, (glyph, 14.0 * zoom), color);
        if enabled && ui.interact(button_rect, id.with(at), Sense::click()).clicked() {
            done.page = Some((list, to));
        }
    }
}

/// A value: its text, or the field it is being typed in.
fn value_ui<S: GraphSource>(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    (input, rect, editable, id): (&NodeInput<'_, S>, Rect, bool, egui::Id),
    (key, text, color): (usize, &str, Color32),
    state: &mut NodeState<'_, S::Node>,
    done: &mut NodeDone<S::Node>,
) {
    let font = egui::FontId::monospace(11.0 * input.zoom);
    let is_this = |editing: &Editing<S::Node>| editing.node == input.node && editing.key == key;

    if let Some(editing) = state.editing.as_mut().filter(|editing| is_this(editing)) {
        let field = egui::TextEdit::singleline(&mut editing.text)
            .font(font)
            .margin(egui::Margin::ZERO)
            .desired_width(rect.width());
        let response = ui.put(rect, field);
        if !editing.focused {
            response.request_focus();
            editing.focused = true;
        }
        if response.lost_focus() {
            let cancelled = ui.input(|input| input.key_pressed(egui::Key::Escape));
            if let Some(editing) = state.editing.take()
                && !cancelled
                && editing.text != text
            {
                done.edit = Some(GraphEdit::Value { node: input.node, key, text: editing.text });
            }
        }
        return;
    }

    let galley = one_row_font(painter, text, font, color, rect.width());
    painter.galley(
        rect.right_center() - egui::vec2(galley.size().x, galley.size().y / 2.0),
        galley,
        Color32::PLACEHOLDER,
    );
    if editable {
        let response = ui.interact(rect, id.with("value"), Sense::click()).on_hover_cursor(egui::CursorIcon::Text);
        if response.clicked() {
            *state.editing = Some(Editing { node: input.node, key, text: text.to_owned(), focused: false });
        }
    }
}

/// A pin: drag it to connect. Filled while connected, in its row's color.
fn pin_ui<S: GraphSource>(
    ui: &egui::Ui,
    painter: &egui::Painter,
    (input, at, editable, id): (&NodeInput<'_, S>, egui::Pos2, bool, egui::Id),
    (key, connected, color): (usize, bool, Color32),
    state: &mut NodeState<'_, S::Node>,
) {
    let radius = PIN * input.zoom;
    let fill = if connected { color } else { ui.visuals().panel_fill };
    painter.circle(at, radius, fill, Stroke::new(1.5, color));
    if !editable {
        return;
    }

    let hit = Rect::from_center_size(at, egui::Vec2::splat((radius * 2.0).max(12.0)));
    let response = ui.interact(hit, id.with("pin"), Sense::drag()).on_hover_cursor(egui::CursorIcon::Grab);
    if response.drag_started() {
        *state.wire = Some((input.index, key));
    }
}

/// `text` at the right of `rect`, centered on its middle.
fn right_text(painter: &egui::Painter, rect: Rect, (text, size): (&str, f32), color: Color32) {
    let galley = one_row(painter, (text, size), color, rect.width());
    painter.galley(
        rect.right_center() - egui::vec2(galley.size().x, galley.size().y / 2.0),
        galley,
        Color32::PLACEHOLDER,
    );
}

/// `text` cut to one row of `width` with `…`, in the proportional font of `size`.
fn one_row(
    painter: &egui::Painter,
    (text, size): (&str, f32),
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    one_row_font(painter, text, egui::FontId::proportional(size.max(1.0)), color, width)
}

fn one_row_font(
    painter: &egui::Painter,
    text: &str,
    font: egui::FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    painter.layout_job(job)
}
