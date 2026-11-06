//! Windows that open next to what was clicked, without covering it.
//!
//! `egui::Window` opens at the top left of the screen, and then wherever it was left: far from what
//! was clicked to open it. Opening at the pointer would cover what was clicked, so:
//! ```text
//!       [button]          below it, if it fits
//!       ┌────────┐
//!       │ window │        else to its right, to its left, above it
//!       └────────┘
//! ```
//! A window's size is only known after it is drawn. So it opens below the pointer, and goes where it
//! fits on the next frames, if that is somewhere else.

use egui::{Pos2, Rect, Vec2, pos2};

/// Default of [`NearPointer::clicked_size`]: about the largest button of the app.
const CLICKED_SIZE: Vec2 = Vec2::new(180.0, 48.0);

/// A window that takes most of the app's, in its center, with the rest dimmed: for what is read at
/// length.
///
/// It closes with its `x`, a click beside it, or `Esc` unless a widget has the keyboard: that key is
/// the widget's, e.g. Vim's in an editor.
#[derive(Debug)]
#[must_use = "Call `show` to draw it"]
pub struct Modal {
    title: egui::WidgetText,
    to_top: bool,
    size: Vec2,
    overlay_alpha: u8,
}

impl Modal {
    #[inline]
    pub fn new(title: impl Into<egui::WidgetText>) -> Self {
        Self { title: title.into(), to_top: false, size: Vec2::new(0.85, 0.85), overlay_alpha: 160 }
    }

    /// Comes above the other windows on this frame, e.g. another modal it was opened from.
    #[inline]
    pub const fn to_top(mut self, to_top: bool) -> Self {
        self.to_top = to_top;
        self
    }

    /// How much of the app's window a [`show_modal`] takes: width, height.
    #[inline]
    pub const fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    /// How dark the backdrop is: `0` shows the app as it is, `255` hides it.
    #[inline]
    pub const fn overlay_alpha(mut self, overlay_alpha: u8) -> Self {
        self.overlay_alpha = overlay_alpha;
        self
    }

    /// Returns what `content` returned, unless it is closed.
    pub fn show<R>(self, ctx: &egui::Context, open: &mut bool, content: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
        if !*open {
            return None;
        }
        let esc = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        // A dialog over it has that `Esc`.
        let esc_is_for_this = ctx.memory(|memory| memory.focused().is_none() && memory.top_modal_layer().is_none());
        if modal_overlay(ctx, self.overlay_alpha).clicked() || (esc && esc_is_for_this) {
            *open = false;
            return None;
        }
        let id = egui::Id::NULL.with(("modal", self.title.text()));
        if self.to_top {
            ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
        }
        let size = ctx.content_rect().size() * self.size;
        egui::Window::new(self.title)
            .id(id)
            .open(open)
            .collapsible(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                let inner = content(ui);
                // A window takes the size of its contents, and is centered with that: however little
                // they are, it stays large and in the center.
                ui.allocate_space(ui.available_size());
                inner
            })
            .and_then(|response| response.inner)
    }
}

/// A [`Modal`] with nothing more to it.
#[inline]
pub fn show_modal<R>(
    ctx: &egui::Context,
    title: impl Into<egui::WidgetText>,
    open: &mut bool,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    Modal::new(title).show(ctx, open, content)
}

/// Draws the dimmed backdrop. The response tells whether it was clicked.
fn modal_overlay(ctx: &egui::Context, alpha: u8) -> egui::Response {
    egui::Area::new(egui::Id::NULL.with("modal_overlay"))
        .order(egui::Order::Background)
        .fixed_pos(ctx.content_rect().min)
        .show(ctx, |ui| {
            let rect = ui.ctx().content_rect();
            ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(alpha));
            ui.allocate_response(rect.size(), egui::Sense::click())
        })
        .inner
}

/// A bar to drag up and down, between two parts of a window. Returns how far it was dragged.
///
/// `height`: what it takes of the layout.
pub fn vertical_splitter(ui: &mut egui::Ui, height: f32) -> f32 {
    let size = egui::vec2(ui.available_width(), height);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
    let response = response.on_hover_cursor(egui::CursorIcon::ResizeVertical);
    let stroke = if response.hovered() || response.dragged() {
        ui.visuals().widgets.hovered.fg_stroke
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    ui.painter().hline(rect.x_range(), rect.center().y, stroke);
    response.drag_delta().y
}

/// A window that goes next to the pointer each time it opens, and stays where it is moved to until it
/// is closed. It stays a little away from the edges of the screen, where its border can be reached.
///
/// "Opens" = it was not drawn in the frame before: closing a window just stops drawing it.
#[inline]
pub fn window_near_pointer<'a>(ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    NearPointer::default().window(ctx, title)
}

/// How [`window_near_pointer`] places a window, when its defaults don't fit.
#[derive(Debug, Clone, Copy)]
#[must_use = "Call `window` to get the window"]
pub struct NearPointer {
    screen_margin: f32,
    clicked_size: Vec2,
    gap: f32,
    placing_frames: u8,
}

impl Default for NearPointer {
    #[inline]
    fn default() -> Self {
        Self { screen_margin: 12.0, clicked_size: CLICKED_SIZE, gap: 4.0, placing_frames: 3 }
    }
}

impl NearPointer {
    /// How far the window stays from the edges of the screen.
    #[inline]
    pub const fn screen_margin(mut self, screen_margin: f32) -> Self {
        self.screen_margin = screen_margin;
        self
    }

    /// What the window should not cover, around the pointer. The widget that was clicked is not known
    /// here, so this is about the largest one.
    #[inline]
    pub const fn clicked_size(mut self, clicked_size: Vec2) -> Self {
        self.clicked_size = clicked_size;
        self
    }

    /// The space between what was clicked and the window.
    #[inline]
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// For how many frames after opening the window is placed: its size settles in the first few.
    #[inline]
    pub const fn placing_frames(mut self, placing_frames: u8) -> Self {
        self.placing_frames = placing_frames;
        self
    }

    /// See [`window_near_pointer`].
    pub fn window<'a>(self, ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
        let title = title.into();
        let id = egui::Id::NULL.with(("window_near_pointer", title.text()));
        let placing_id = id.with("placing");
        let frame = ctx.cumulative_pass_nr();
        let last_drawn = ctx.data_mut(|data| data.get_temp::<u64>(id));
        ctx.data_mut(|data| data.insert_temp(id, frame));

        let screen = ctx.content_rect().shrink(self.screen_margin);
        let window = egui::Window::new(title).id(id).constrain_to(screen);
        if just_opened(last_drawn, frame) {
            let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) else { return window };
            ctx.data_mut(|data| data.insert_temp(placing_id, (pointer, self.placing_frames)));
            return window.current_pos(self.place(pointer, Vec2::ZERO, screen));
        }

        let placing = ctx.data_mut(|data| data.get_temp::<(Pos2, u8)>(placing_id));
        let (Some((pointer, frames_left)), Some(rect)) = (placing, ctx.memory(|memory| memory.area_rect(id))) else {
            return window;
        };
        ctx.data_mut(|data| {
            if frames_left > 1 {
                data.insert_temp(placing_id, (pointer, frames_left - 1));
            } else {
                data.remove_temp::<(Pos2, u8)>(placing_id);
            }
        });
        ctx.request_repaint();
        window.current_pos(self.place(pointer, rect.size(), screen))
    }

    /// Where a window of `size` goes (its top left), with `pointer` at what was clicked. See the module
    /// docs. If it fits nowhere: below, moved up as far as it must, as egui would.
    fn place(self, pointer: Pos2, size: Vec2, screen: Rect) -> Pos2 {
        let clicked = Rect::from_center_size(pointer, self.clicked_size).expand(self.gap);
        let fits = |at: Pos2| screen.contains_rect(Rect::from_min_size(at, size));
        // Along the side it opens at, it moves as little as it must to stay on the screen.
        let x = pointer.x.min(screen.right() - size.x).max(screen.left());
        let y = pointer.y.min(screen.bottom() - size.y).max(screen.top());

        let below = pos2(x, clicked.bottom());
        let right = pos2(clicked.right(), y);
        let left = pos2(clicked.left() - size.x, y);
        let above = pos2(x, clicked.top() - size.y);
        [below, right, left, above].into_iter().find(|at| fits(*at)).unwrap_or_else(|| pos2(x, y.min(below.y)))
    }
}

/// See [`window_near_pointer`]. `last_drawn`: the frame it was last drawn in, if any.
fn just_opened(last_drawn: Option<u64>, frame: u64) -> bool {
    last_drawn.is_none_or(|last| last + 1 < frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(1000.0, 800.0) };

    /// Draws a modal for a frame, with `events`. Returns what it shows, if it does.
    fn modal_frame(ctx: &egui::Context, open: &mut bool, events: Vec<egui::Event>) -> Option<Rect> {
        let input = egui::RawInput { screen_rect: Some(SCREEN), events, ..Default::default() };
        let mut shown = None;
        let mut output = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            shown = show_modal(&ctx, "Modal", open, |ui| ui.max_rect());
        });
        output.textures_delta.clear(); // egui insists on them being taken
        shown
    }

    #[test]
    fn a_modal_is_large_and_in_the_center() {
        let ctx = egui::Context::default();
        let mut open = true;
        for _ in 0..3 {
            modal_frame(&ctx, &mut open, Vec::new());
        }
        let rect = modal_frame(&ctx, &mut open, Vec::new()).unwrap();
        // The title bar and the frame are around what `content` is given.
        assert!((rect.center() - SCREEN.center()).length() < 24.0, "{rect:?}");
        assert!((rect.width() - 850.0).abs() < 24.0, "{rect:?}");
        assert!((rect.height() - 680.0).abs() < 60.0, "{rect:?}");
        assert!(open);
    }

    #[test]
    fn a_modal_closes_with_a_click_beside_it_or_esc() {
        let click = |pressed| egui::Event::PointerButton {
            pos: pos2(5.0, 5.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let ctx = egui::Context::default();
        let mut open = true;
        modal_frame(&ctx, &mut open, Vec::new());
        modal_frame(&ctx, &mut open, vec![egui::Event::PointerMoved(pos2(5.0, 5.0))]);
        modal_frame(&ctx, &mut open, vec![click(true)]);
        modal_frame(&ctx, &mut open, vec![click(false)]);
        modal_frame(&ctx, &mut open, Vec::new());
        assert!(!open, "a click beside it");

        let mut open = true;
        let esc = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        modal_frame(&ctx, &mut open, vec![esc]);
        assert!(!open, "Esc");
        assert!(modal_frame(&ctx, &mut open, Vec::new()).is_none(), "closed: nothing is drawn");
    }
    const SIZE: Vec2 = Vec2::new(300.0, 200.0);

    fn place(pointer: Pos2, size: Vec2, screen: Rect) -> Pos2 {
        NearPointer::default().place(pointer, size, screen)
    }

    #[test]
    fn the_gap_and_what_is_not_covered_can_be_changed() {
        let pointer = pos2(400.0, 100.0);
        let at = NearPointer::default().gap(10.0).clicked_size(Vec2::new(100.0, 100.0)).place(pointer, SIZE, SCREEN);
        assert_eq!(at, pos2(400.0, 160.0)); // 100 + 100 / 2 + 10
    }

    fn covers_the_click(at: Pos2, pointer: Pos2) -> bool {
        Rect::from_min_size(at, SIZE).intersects(Rect::from_center_size(pointer, CLICKED_SIZE))
    }

    #[test]
    fn a_window_opens_after_a_frame_without_it() {
        assert!(just_opened(None, 10));
        assert!(just_opened(Some(5), 10));
        assert!(!just_opened(Some(9), 10)); // still open
        assert!(!just_opened(Some(10), 10)); // a second pass of the same frame
    }

    #[test]
    fn below_what_was_clicked_if_it_fits() {
        let pointer = pos2(400.0, 100.0);
        let at = place(pointer, SIZE, SCREEN);
        assert_eq!(at, pos2(400.0, 128.0));
        assert!(!covers_the_click(at, pointer));
    }

    #[test]
    fn at_the_bottom_of_the_screen_it_goes_to_the_side() {
        let pointer = pos2(400.0, 760.0);
        let at = place(pointer, SIZE, SCREEN);
        assert!(at.x > pointer.x, "to the right: {at:?}");
        assert!(!covers_the_click(at, pointer));

        // No room on the right either.
        let corner = pos2(900.0, 760.0);
        let at = place(corner, SIZE, SCREEN);
        assert!(at.x < corner.x, "to the left: {at:?}");
        assert!(!covers_the_click(at, corner));
    }

    #[test]
    fn near_the_right_edge_it_stays_on_the_screen() {
        let pointer = pos2(950.0, 100.0);
        let at = place(pointer, SIZE, SCREEN);
        assert!(SCREEN.contains_rect(Rect::from_min_size(at, SIZE)));
        assert!(!covers_the_click(at, pointer));
    }

    #[test]
    fn a_window_too_big_for_any_side_is_still_placed() {
        let huge = Vec2::new(900.0, 780.0);
        let at = place(pos2(500.0, 400.0), huge, SCREEN);
        assert!(SCREEN.contains_rect(Rect::from_min_size(at, huge)));
    }
}
