//! Rows of one height in a list that scrolls only up and down, as VS Code's explorer:
//!
//! ```text
//! let height = list::row_height(ui);
//! let mut scroll = egui::ScrollArea::vertical();
//! if let Some(y) = list::offset_to_show(cursor, (height, spacing), rows.len(), state.scroll, 0) {
//!     scroll = scroll.vertical_scroll_offset(y);      // just enough to show the cursor's row
//! }
//! let output = scroll.show_rows(ui, height, rows.len(), |ui, range| {
//!     for row in range { list::fixed_row(ui, height, |ui| { /* the row */ }); }
//! });
//! state.scroll.note(&output);
//! ```
//! A row is cut at the list's width, so a long one never widens the panel (cut its text with `…`
//! first, e.g. `egui_widgets::fit`). Sideways scrolling is left out on purpose: `show_rows` draws only
//! the rows seen, so the content's width would change with them and the scroll bars come and go.

/// Where a list of rows was scrolled when last drawn. See [`offset_to_show`].
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct ListScroll {
    /// The vertical offset, e.g. `120.0`.
    pub offset: f32,
    /// The height of what is seen of the list.
    pub view: f32,
}

impl ListScroll {
    /// Notes where the list is now, from its `show_rows`.
    #[inline]
    pub fn note<R>(&mut self, output: &egui::scroll_area::ScrollAreaOutput<R>) {
        (self.offset, self.view) = (output.state.clamped_offset().y, output.inner_rect.height());
    }
}

/// The height of a row: each is drawn exactly that high ([`fixed_row`]), or `show_rows` would place
/// them, and scroll to them, wrongly.
#[inline]
#[must_use]
pub fn row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Monospace) + 4.0)
}

/// Draws a row of `height` and the list's width exactly, whatever is in it, left to right and
/// centered. What goes past the width is cut: a row a pixel wider would widen the panel, then the
/// rows, and so on.
pub fn fixed_row<R>(ui: &mut egui::Ui, height: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width().max(1.0), height));

    let inner = {
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        let mut row = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
        row.set_clip_rect(rect.intersect(ui.clip_rect()));
        add(&mut row)
    };

    ui.allocate_rect(rect, egui::Sense::hover());
    inner
}

/// The frame of the row the keyboard is on, around `rect`, in the selection's color.
#[inline]
pub fn cursor_frame(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().rect_stroke(rect, 2.0, ui.visuals().selection.stroke, egui::StrokeKind::Inside);
}

/// The offset that shows `row` whole; `None`: it is seen already.
///
/// As VS Code's explorer reveals a file: scrolled only as much as it needs, below the `covered` rows
/// pinned over the top (a tree's sticky parents), and never past the ends, where the list would
/// spring back and shake.
///
/// `(row_height, spacing)`: as given to `show_rows`; `rows`: how many there are.
///
/// e.g. rows of 20 px (no spacing), 100 px seen, at 0: row 7 -> `Some(60.0)` (rows 3..=7 seen); row 2
/// -> `None`; with 2 rows covering the top at 60: row 4 -> `Some(40.0)`.
#[must_use]
pub fn offset_to_show(
    row: usize,
    (row_height, spacing): (f32, f32),
    rows: usize,
    scroll: ListScroll,
    covered: usize,
) -> Option<f32> {
    #[expect(clippy::cast_precision_loss, reason = "far fewer rows than f32 can count")]
    let (row, rows, covered) = (row as f32, rows as f32, covered as f32);
    let step = row_height + spacing;

    // Up to its top below what is pinned, or down to its bottom.
    let wanted = {
        let (top, pinned) = (row * step, covered * step);
        if top - pinned < scroll.offset {
            top - pinned
        } else if top + row_height > scroll.offset + scroll.view {
            top + row_height - scroll.view
        } else {
            return None;
        }
    };

    // Never past the ends. As `show_rows` makes the content: no spacing after the last row.
    let max = rows.mul_add(step, -spacing).max(0.0) - scroll.view;
    let wanted = wanted.min(max).max(0.0);

    ((wanted - scroll.offset).abs() >= 0.5).then_some(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only as much as the row needs, below what is pinned, and never past the ends.
    #[test]
    fn a_row_is_scrolled_into_view() {
        let at = |offset| ListScroll { offset, view: 100.0 };
        assert_eq!(offset_to_show(7, (20.0, 0.0), 50, at(0.0), 0), Some(60.0), "below: it comes to the bottom");
        assert_eq!(offset_to_show(2, (20.0, 0.0), 50, at(0.0), 0), None, "seen already");
        assert_eq!(offset_to_show(1, (20.0, 0.0), 50, at(60.0), 0), Some(20.0), "above: it comes to the top");
        assert_eq!(offset_to_show(4, (20.0, 0.0), 50, at(60.0), 2), Some(40.0), "under the pinned rows");
        assert_eq!(offset_to_show(0, (20.0, 0.0), 50, at(60.0), 2), Some(0.0), "not above the start");
        // 10 rows of 20 px with 4 px between: 236 px, 136 px at most.
        assert_eq!(offset_to_show(9, (20.0, 4.0), 10, at(0.0), 0), Some(136.0), "not past the end");
        assert_eq!(offset_to_show(9, (20.0, 4.0), 10, at(136.0), 0), None, "at the end already");
    }
}
