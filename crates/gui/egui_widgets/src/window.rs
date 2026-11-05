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

/// How far a window stays from the edges of the screen.
const SCREEN_MARGIN: f32 = 12.0;

/// What the window should not cover, around the pointer: about the largest button of the app. The
/// widget that was clicked is not known here.
const CLICKED_SIZE: Vec2 = Vec2::new(180.0, 48.0);

/// The space between what was clicked and the window.
const GAP: f32 = 4.0;

/// For how many frames after opening a window is placed: its size settles in the first few.
const PLACING_FRAMES: u8 = 3;

/// A window that goes next to the pointer each time it opens, and stays where it is moved to until it
/// is closed. It stays a little away from the edges of the screen, where its border can be reached.
///
/// "Opens" = it was not drawn in the frame before: closing a window just stops drawing it.
pub fn window_near_pointer<'a>(ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    let title = title.into();
    let id = egui::Id::NULL.with(("window_near_pointer", title.text()));
    let placing_id = id.with("placing");
    let frame = ctx.cumulative_pass_nr();
    let last_drawn = ctx.data_mut(|data| data.get_temp::<u64>(id));
    ctx.data_mut(|data| data.insert_temp(id, frame));

    let screen = ctx.content_rect().shrink(SCREEN_MARGIN);
    let window = egui::Window::new(title).id(id).constrain_to(screen);
    if just_opened(last_drawn, frame) {
        let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) else { return window };
        ctx.data_mut(|data| data.insert_temp(placing_id, (pointer, PLACING_FRAMES)));
        return window.current_pos(place(pointer, Vec2::ZERO, screen));
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
    window.current_pos(place(pointer, rect.size(), screen))
}

/// See [`window_near_pointer`]. `last_drawn`: the frame it was last drawn in, if any.
fn just_opened(last_drawn: Option<u64>, frame: u64) -> bool {
    last_drawn.is_none_or(|last| last + 1 < frame)
}

/// Where a window of `size` goes (its top left), with `pointer` at what was clicked. See the module
/// docs. If it fits nowhere: below, moved up as far as it must, as egui would.
fn place(pointer: Pos2, size: Vec2, screen: Rect) -> Pos2 {
    let clicked = Rect::from_center_size(pointer, CLICKED_SIZE).expand(GAP);
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

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Rect = Rect { min: pos2(0.0, 0.0), max: pos2(1000.0, 800.0) };
    const SIZE: Vec2 = Vec2::new(300.0, 200.0);

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
