//! Windows that open where you clicked.
//!
//! `egui::Window` opens at the top left of the screen, and then wherever it was left: far from what
//! was clicked to open it.

/// A window that goes to the pointer each time it opens, and stays where it is moved to until it is
/// closed. egui keeps it on the screen.
///
/// "Opens" = it was not drawn in the frame before: closing a window just stops drawing it.
pub fn window_near_pointer<'a>(ctx: &egui::Context, title: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    let title = title.into();
    let id = egui::Id::NULL.with(("window_near_pointer", title.text()));
    let frame = ctx.cumulative_pass_nr();
    let last_drawn = ctx.data_mut(|data| data.get_temp::<u64>(id));
    ctx.data_mut(|data| data.insert_temp(id, frame));

    let window = egui::Window::new(title);
    if just_opened(last_drawn, frame)
        && let Some(pointer) = ctx.input(|input| input.pointer.latest_pos())
    {
        return window.current_pos(pointer);
    }
    window
}

/// See [`window_near_pointer`]. `last_drawn`: the frame it was last drawn in, if any.
fn just_opened(last_drawn: Option<u64>, frame: u64) -> bool {
    last_drawn.is_none_or(|last| last + 1 < frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_opens_after_a_frame_without_it() {
        assert!(just_opened(None, 10));
        assert!(just_opened(Some(5), 10));
        assert!(!just_opened(Some(9), 10)); // still open
        assert!(!just_opened(Some(10), 10)); // a second pass of the same frame
    }
}
