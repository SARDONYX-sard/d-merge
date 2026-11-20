//! Draws a [`Dock`]: its main area in the host's `Ui`, each window as a viewport of its own, and the
//! drag of a tab between groups.
//!
//! ```text
//! ┌ group (the host's) ──────────┐
//! │ [Root] [Idle] [Preview]      │  <- the host's own tab bar: `TabDrags::bar`, and each tab's
//! ├──────────────────────────────┤     drag through `TabDrags::tab`
//! │░░░░░░│                │░░░░░░│  <- dropped on an edge (a quarter): beside the group there
//! │░░░░░░│     middle     │░░░░░░│     in the middle or on its bar: among its tabs
//! └──────────────────────────────┘
//! ```
//!
//! A drag can leave the window it began in: the pointer is followed on the screen (the window's
//! place on it, plus the pointer in it), so that the group under it is found in any window; dropped
//! where no area is, it goes to a window of its own there. What is dropped where is the host's to
//! carry out: see [`Dropped`].
//!
//! What each group and area was is kept from one frame to the next, for the drag to find where it
//! is over: what it shows is a frame behind, too little to see.

use egui::{Id, Pos2, Rect, Sense, Vec2};

use crate::{
    GroupViewer,
    dock::{Area, Dock, Place, Target},
    node::{Axis, Half, Node},
};

/// The width of the handle between the halves of a split.
const HANDLE: f32 = 5.0;
/// How much of a group, from an edge, puts a tab dropped there beside it.
const EDGE: f32 = 0.25;
/// The smallest share of a split's half.
const MIN_FRACTION: f32 = 0.1;
/// The size of a window of its own, when the group had no room to measure.
const WINDOW_SIZE: Vec2 = Vec2::new(640.0, 480.0);

/// A tab dropped: the host moves it, from the group at `from`, to `target`.
#[derive(Debug, Clone, PartialEq)]
pub struct Dropped<P> {
    pub item: P,
    pub from: Place,
    pub target: Target,
}

/// What [`DockArea::show_inside`] leaves to the host.
#[derive(Debug)]
pub struct DockOutput<G, P> {
    pub dropped: Option<Dropped<P>>,
    /// The groups of the windows the user closed: the host takes their tabs in.
    pub closed: Vec<Node<G>>,
}

/// Draws a [`Dock`]. See the module docs.
#[must_use = "Call `show_inside` to draw it"]
pub struct DockArea<'a, G, P> {
    dock: &'a mut Dock<G, P>,
    id: Id,
}

impl<G, P> std::fmt::Debug for DockArea<'_, G, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockArea").field("id", &self.id).finish_non_exhaustive()
    }
}

/// A group as it was drawn: in the coordinates of its viewport.
#[derive(Debug, Clone)]
struct DrawnGroup {
    place: Place,
    rect: Rect,
    /// The host's tab bar, if it told it ([`TabDrags::bar`]).
    bar: Option<Rect>,
    /// The middle of each tab in the bar, left to right.
    tab_middles: Vec<f32>,
}

/// An area as it was drawn: where it is on the screen, and where its viewport's origin is.
#[derive(Debug, Clone, Copy)]
struct DrawnArea {
    area: Area,
    /// On the screen.
    rect: Rect,
    /// Of its viewport, on the screen: a point of it is `origin + pos` there.
    origin: Vec2,
}

/// A tab being dragged.
#[derive(Debug)]
struct Drag<P> {
    item: P,
    from: Place,
    title: String,
    /// Of its group: the size of a window it opens in.
    size: Vec2,
    /// See [`TabDrags::tab`].
    can_leave: bool,
    /// The pointer on the screen.
    pointer: Option<Pos2>,
    /// Where it would go now.
    target: Option<Target>,
}

/// Kept in the [`Dock`] from one frame to the next. See the module docs.
#[derive(Debug)]
pub(crate) struct Runtime<P> {
    drag: Option<Drag<P>>,
    groups: Vec<DrawnGroup>,
    areas: Vec<DrawnArea>,
}

impl<P> Default for Runtime<P> {
    #[inline]
    fn default() -> Self {
        Self { drag: None, groups: Vec::new(), areas: Vec::new() }
    }
}

/// Given to the host's group as it is drawn: its tabs are dragged through it.
#[derive(Debug)]
pub struct TabDrags<'a, P> {
    drag: &'a mut Option<Drag<P>>,
    drawn: &'a mut DrawnGroup,
}

impl<P> TabDrags<'_, P> {
    /// A tab of the group's bar: dragged, it takes `item` along (`title`: drawn at the pointer).
    /// `can_leave`: it may go to a window of its own (else only among the main area's groups).
    pub fn tab(&mut self, response: &egui::Response, title: &str, can_leave: bool, item: impl FnOnce() -> P) {
        self.drawn.tab_middles.push(response.rect.center().x);
        if response.drag_started() && self.drag.is_none() {
            let (from, size) = (self.drawn.place.clone(), self.drawn.rect.size());
            let title = title.to_owned();
            *self.drag = Some(Drag { item: item(), from, title, size, can_leave, pointer: None, target: None });
        }
    }

    /// The group's tab bar: a tab dropped on it goes among the tabs.
    #[inline]
    pub const fn bar(&mut self, rect: Rect) {
        self.drawn.bar = Some(rect);
    }

    /// A tab of some group is being dragged.
    #[inline]
    #[must_use]
    pub const fn dragging(&self) -> bool {
        self.drag.is_some()
    }
}

/// What one frame's drawing finds along the way.
struct Frame<P> {
    id: Id,
    runtime: Runtime<P>,
    groups: Vec<DrawnGroup>,
    areas: Vec<DrawnArea>,
    /// The drag ended this frame.
    released: bool,
}

impl<'a, G, P> DockArea<'a, G, P> {
    /// `id_salt`: one per dock, e.g. per window of the host it is in.
    #[inline]
    pub fn new(id_salt: impl egui::AsIdSalt, dock: &'a mut Dock<G, P>) -> Self {
        Self { dock, id: Id::NULL.with(id_salt) }
    }

    /// Draws the main area in all the room of `ui`, then each window of its own. Returns a tab
    /// dropped, and the groups of closed windows: the host's to carry out.
    pub fn show_inside<V: GroupViewer<Group = G, Item = P>>(
        self,
        ui: &mut egui::Ui,
        viewer: &mut V,
    ) -> DockOutput<G, P> {
        let Self { dock, id } = self;
        let runtime = std::mem::take(&mut dock.runtime);
        let mut frame = Frame { id, runtime, groups: Vec::new(), areas: Vec::new(), released: false };

        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, Sense::hover());
        frame.viewport(ui, Area::Main, rect);
        node_ui(ui, (&mut frame, Place::default()), dock.main_mut(), rect, viewer);

        let closed = windows_ui(ui.ctx(), (&mut dock.windows, &mut frame), viewer);
        let closed = closed.into_iter().filter_map(|window| dock.close_window(window)).collect();

        let dropped = frame.finish(ui.ctx());
        dock.runtime = frame.runtime;
        DockOutput { dropped, closed }
    }
}

/// Each window of its own, as a viewport. Returns those the user closed.
fn windows_ui<G, P, V: GroupViewer<Group = G, Item = P>>(
    ctx: &egui::Context,
    (windows, frame): (&mut [crate::Window<G>], &mut Frame<P>),
    viewer: &mut V,
) -> Vec<u64> {
    let mut closed = Vec::new();
    for window in windows {
        let title = window.node.groups().first().map(|(_, group)| viewer.title(group)).unwrap_or_default();
        let builder = egui::ViewportBuilder::default()
            .with_title(title)
            .with_position(window.opened_at.min)
            .with_inner_size(window.opened_at.size());
        let viewport = egui::ViewportId::from_hash_of((frame.id, window.id));
        let area = Area::Window(window.id);
        ctx.show_viewport_immediate(viewport, builder, |ui, _class| {
            let rect = ui.available_rect_before_wrap();
            ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
            ui.allocate_rect(rect, Sense::hover());
            frame.viewport(ui, area, rect);
            node_ui(ui, (frame, Place::new(area, Vec::new())), &mut window.node, rect, viewer);
            if ui.input(|input| input.viewport().close_requested()) {
                closed.push(window.id);
            }
        });
        if frame.runtime.drag.is_some() {
            ctx.request_repaint_of(viewport);
        }
    }
    closed
}

/// A node in `rect`: a split's halves and its handle, or a group.
fn node_ui<G, P, V: GroupViewer<Group = G, Item = P>>(
    ui: &mut egui::Ui,
    (frame, place): (&mut Frame<P>, Place),
    node: &mut Node<G>,
    rect: Rect,
    viewer: &mut V,
) {
    match node {
        Node::Group(group) => group_ui(ui, (frame, place), group, rect, viewer),
        Node::Split(split) => {
            let (first, handle, second) = halves(rect, split.axis, split.fraction);
            let id = frame.id.with(("dock_split", place.area, &place.path));
            if let Some(fraction) = handle_ui(ui, id, (handle, rect), split.axis) {
                split.fraction = fraction;
            }
            node_ui(ui, (frame, place.child(Half::First)), &mut split.first, first, viewer);
            node_ui(ui, (frame, place.child(Half::Second)), &mut split.second, second, viewer);
        }
    }
}

/// `rect` cut along `axis`: the first half (`fraction` of it), the handle, the second half.
fn halves(rect: Rect, axis: Axis, fraction: f32) -> (Rect, Rect, Rect) {
    let along = match axis {
        Axis::Horizontal => 0,
        Axis::Vertical => 1,
    };
    let cut = (rect.size()[along] - HANDLE).mul_add(fraction, rect.min[along]);
    let mut first = rect;
    first.max[along] = cut;
    let mut handle = rect;
    (handle.min[along], handle.max[along]) = (cut, cut + HANDLE);
    let mut second = rect;
    second.min[along] = cut + HANDLE;
    (first, handle, second)
}

/// The handle between a split's halves: dragged, the share of the first half (`rect`: the split's).
fn handle_ui(ui: &egui::Ui, id: Id, (handle, rect): (Rect, Rect), axis: Axis) -> Option<f32> {
    let (along, cursor) = match axis {
        Axis::Horizontal => (0, egui::CursorIcon::ResizeHorizontal),
        Axis::Vertical => (1, egui::CursorIcon::ResizeVertical),
    };
    let response = ui.interact(handle, id, Sense::drag()).on_hover_cursor(cursor);
    let visuals = ui.visuals();
    let stroke = if response.hovered() || response.dragged() {
        visuals.widgets.hovered.fg_stroke
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    let middle = handle.center();
    match axis {
        Axis::Horizontal => _ = ui.painter().vline(middle.x, handle.y_range(), stroke),
        Axis::Vertical => _ = ui.painter().hline(handle.x_range(), middle.y, stroke),
    }
    let pointer = response.interact_pointer_pos().filter(|_| response.dragged())?;
    let share = (pointer[along] - rect.min[along]) / rect.size()[along].max(1.0);
    Some(share.clamp(MIN_FRACTION, 1.0 - MIN_FRACTION))
}

/// A group: the host draws it all, its tab bar too, telling its tabs through [`TabDrags`].
fn group_ui<G, P, V: GroupViewer<Group = G, Item = P>>(
    ui: &mut egui::Ui,
    (frame, place): (&mut Frame<P>, Place),
    group: &mut G,
    rect: Rect,
    viewer: &mut V,
) {
    let mut drawn = DrawnGroup { place, rect, bar: None, tab_middles: Vec::new() };
    let id = frame.id.with(("dock_group", drawn.place.area, &drawn.place.path));
    let mut group_ui = ui.new_child(egui::UiBuilder::new().id_salt(id).max_rect(rect));
    group_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    let mut drags = TabDrags { drag: &mut frame.runtime.drag, drawn: &mut drawn };
    viewer.ui(&mut group_ui, group, &mut drags);

    drop_marker(ui, frame.runtime.drag.as_ref(), &drawn);
    frame.groups.push(drawn);
}

/// Where a dragged tab would go, if onto this group: shaded.
fn drop_marker<P>(ui: &egui::Ui, drag: Option<&Drag<P>>, group: &DrawnGroup) {
    let shade = match drag.and_then(|drag| drag.target.as_ref()) {
        Some(Target::Join(place, _)) if *place == group.place => group.rect,
        Some(Target::Beside(place, axis, half)) if *place == group.place => match (axis, half) {
            (Axis::Horizontal, Half::First) => group.rect.split_left_right_at_fraction(0.5).0,
            (Axis::Horizontal, Half::Second) => group.rect.split_left_right_at_fraction(0.5).1,
            (Axis::Vertical, Half::First) => group.rect.split_top_bottom_at_fraction(0.5).0,
            (Axis::Vertical, Half::Second) => group.rect.split_top_bottom_at_fraction(0.5).1,
        },
        _ => return,
    };
    let layer = egui::LayerId::new(egui::Order::Foreground, Id::NULL.with("dock_drop_marker"));
    let color = ui.visuals().selection.bg_fill.gamma_multiply(0.35);
    ui.ctx().layer_painter(layer).rect_filled(shade, 4.0, color);
}

/// Where a tab dropped at `at` (in the group's viewport) goes, if over `group`.
fn drop_on(group: &DrawnGroup, at: Pos2) -> Option<Target> {
    let join = |index| Target::Join(group.place.clone(), index);
    if group.bar.is_some_and(|bar| bar.contains(at)) {
        return Some(join(group.tab_middles.iter().filter(|&&middle| middle < at.x).count()));
    }
    if !group.rect.contains(at) {
        return None;
    }
    let share = (at - group.rect.min) / group.rect.size();
    let beside = |axis, half| Target::Beside(group.place.clone(), axis, half);
    Some(if share.x < EDGE {
        beside(Axis::Horizontal, Half::First)
    } else if share.x > 1.0 - EDGE {
        beside(Axis::Horizontal, Half::Second)
    } else if share.y < EDGE {
        beside(Axis::Vertical, Half::First)
    } else if share.y > 1.0 - EDGE {
        beside(Axis::Vertical, Half::Second)
    } else {
        join(group.tab_middles.len())
    })
}

impl<P> Frame<P> {
    /// An area drawn in `rect` of `ui`'s viewport: where it is on the screen. While a tab is dragged
    /// in this viewport, the pointer is followed there, its title drawn along, and its release
    /// noted.
    fn viewport(&mut self, ui: &egui::Ui, area: Area, rect: Rect) {
        let origin = ui.input(|input| input.viewport().inner_rect).map_or(Vec2::ZERO, |inner| inner.min.to_vec2());
        self.areas.push(DrawnArea { area, rect: rect.translate(origin), origin });

        let Some(drag) = &mut self.runtime.drag else { return };
        let (down, released, pointer) = ui.input(|input| {
            (input.pointer.primary_down(), input.pointer.primary_released(), input.pointer.latest_pos())
        });
        // Only the viewport the drag began in hears the pointer while the button is held.
        if let Some(pointer) = pointer.filter(|_| down || released) {
            drag.pointer = Some(pointer + origin);
            ghost(ui, pointer, &drag.title);
            ui.ctx().request_repaint();
        }
        self.released |= released;
    }

    /// The frame's areas and groups are what the drag is found over now. Returns a drop.
    fn finish(&mut self, ctx: &egui::Context) -> Option<Dropped<P>> {
        self.runtime.groups = std::mem::take(&mut self.groups);
        self.runtime.areas = std::mem::take(&mut self.areas);
        let Runtime { drag, groups, areas } = &mut self.runtime;
        let drag_now = drag.as_mut()?;
        let target = drag_now.pointer.map(|pointer| target_at(areas, groups, pointer, drag_now.size));
        // One that cannot leave the main area goes nowhere else.
        let stays = |target: &Target| match target {
            Target::Join(place, _) | Target::Beside(place, ..) => place.area == Area::Main,
            Target::NewWindow(_) => false,
        };
        drag_now.target = target.filter(|target| drag_now.can_leave || stays(target));
        if !self.released {
            ctx.request_repaint();
            return None;
        }
        let Drag { item, from, target, .. } = drag.take()?;
        target.map(|target| Dropped { item, from, target })
    }
}

/// Where a tab dropped at `pointer` on the screen goes: onto the group of the area under it (a
/// window first: it is above the main area), or else into a window of its own there.
fn target_at(areas: &[DrawnArea], groups: &[DrawnGroup], pointer: Pos2, size: Vec2) -> Target {
    let under = areas.iter().rev().find(|drawn| drawn.rect.contains(pointer));
    let found = under.and_then(|drawn| {
        let at = pointer - drawn.origin;
        groups.iter().filter(|group| group.place.area == drawn.area).find_map(|group| drop_on(group, at))
    });
    let size = if size.x > 100.0 && size.y > 100.0 { size } else { WINDOW_SIZE };
    found.unwrap_or_else(|| Target::NewWindow(Rect::from_min_size(pointer - egui::vec2(40.0, 12.0), size)))
}

/// The dragged tab's title at the pointer.
fn ghost(ui: &egui::Ui, pointer: Pos2, title: &str) {
    let layer = egui::LayerId::new(egui::Order::Tooltip, Id::NULL.with("dock_drag_ghost"));
    let painter = ui.ctx().layer_painter(layer);
    let visuals = ui.visuals();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = painter.layout_no_wrap(title.to_owned(), font, visuals.text_color());
    let rect = Rect::from_min_size(pointer + egui::vec2(12.0, 8.0), galley.size() + egui::vec2(16.0, 8.0));
    painter.rect_filled(rect, 4.0, visuals.selection.bg_fill.gamma_multiply(0.8));
    painter.galley(rect.min + egui::vec2(8.0, 4.0), galley, visuals.text_color());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(place: Place, rect: Rect, bar: Rect) -> DrawnGroup {
        DrawnGroup { place, rect, bar: Some(bar), tab_middles: vec![20.0, 60.0] }
    }

    /// On the bar: among the tabs, by the pointer's place among their middles; on an edge: beside
    /// the group there; in its middle: after its last tab.
    #[test]
    fn a_drop_goes_by_where_it_is() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 400.0));
        let drawn = group(Place::default(), rect, rect.with_max_y(26.0));
        let at = |x, y| drop_on(&drawn, egui::pos2(x, y));
        let beside = |axis, half| Some(Target::Beside(Place::default(), axis, half));
        assert_eq!(at(40.0, 10.0), Some(Target::Join(Place::default(), 1)));
        assert_eq!(at(10.0, 200.0), beside(Axis::Horizontal, Half::First));
        assert_eq!(at(390.0, 200.0), beside(Axis::Horizontal, Half::Second));
        assert_eq!(at(200.0, 390.0), beside(Axis::Vertical, Half::Second));
        assert_eq!(at(200.0, 200.0), Some(Target::Join(Place::default(), 2)));
        assert_eq!(at(500.0, 200.0), None);
    }

    /// Over a window (on the screen, through its viewport's origin), onto its group; over no area,
    /// a window of its own there.
    #[test]
    fn a_drop_finds_its_window_on_the_screen() {
        let local = Rect::from_min_size(Pos2::ZERO, egui::vec2(400.0, 300.0));
        let origin = egui::vec2(1000.0, 100.0);
        let window = Area::Window(1);
        let areas = [
            DrawnArea { area: Area::Main, rect: local, origin: Vec2::ZERO },
            DrawnArea { area: window, rect: local.translate(origin), origin },
        ];
        let groups = [group(Place::new(window, Vec::new()), local, local.with_max_y(26.0))];

        let target = target_at(&areas, &groups, egui::pos2(1200.0, 260.0), egui::vec2(300.0, 200.0));
        assert_eq!(target, Target::Join(Place::new(window, Vec::new()), 2), "its middle: among its tabs");
        let outside = target_at(&areas, &groups, egui::pos2(700.0, 700.0), egui::vec2(300.0, 200.0));
        assert!(matches!(outside, Target::NewWindow(rect) if rect.size() == egui::vec2(300.0, 200.0)));
    }

    #[test]
    fn a_split_cuts_its_rect_with_a_handle_between() {
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(105.0, 50.0));
        let (first, handle, second) = halves(rect, Axis::Horizontal, 0.4);
        assert_eq!((first.width(), handle.width(), second.width()), (40.0, HANDLE, 60.0));
    }
}
