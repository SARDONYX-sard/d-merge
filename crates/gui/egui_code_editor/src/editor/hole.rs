//! A hole in what the editor drew, where something floats over it with a see-through background (the
//! references' peek): what is under it is not drawn, so the page shows through, not the text.
//!
//! ```text
//! the editor's shapes, each with its clip rect      after the hole is punched
//! ┌────────────────────────┐                        ┌────────────────────────┐
//! │ <hkobject ...>         │                        │ <hkobject ...>         │  above: kept
//! │ <hkparam ...>          │   + hole ┌──────┐  ->   ├──────────┐ ┌──────────┤
//! │ <hkparam ...>          │          └──────┘      │ <hkparam │ │ ...>     │  beside: kept
//! │ </hkobject>            │                        ├──────────┘ └──────────┤
//! └────────────────────────┘                        │ </hkobject>            │  below: kept
//!                                                   └────────────────────────┘
//! ```
//!
//! egui clips each shape to a rect, no more: a shape over the hole is drawn up to four times, each
//! clipped to a part of its clip rect around the hole. The text keeps its place: nothing moves.

use egui::{LayerId, Rect};

/// The parts of `clip` around `hole`: above, below, left and right of it (each between the other
/// two), those not empty. `clip` itself if they don't meet.
///
/// e.g. `clip` 0..100 × 0..100, `hole` 0..100 × 40..60: `0..100 × 0..40` and `0..100 × 60..100`.
pub(crate) fn around(clip: Rect, hole: Rect) -> Vec<Rect> {
    let cut = clip.intersect(hole);
    if !cut.is_positive() {
        return vec![clip];
    }
    let parts = [
        Rect::from_min_max(clip.min, egui::pos2(clip.max.x, cut.min.y)),
        Rect::from_min_max(egui::pos2(clip.min.x, cut.max.y), clip.max),
        Rect::from_min_max(egui::pos2(clip.min.x, cut.min.y), egui::pos2(cut.min.x, cut.max.y)),
        Rect::from_min_max(egui::pos2(cut.max.x, cut.min.y), egui::pos2(clip.max.x, cut.max.y)),
    ];
    parts.into_iter().filter(Rect::is_positive).collect()
}

/// Punches `hole` in the shapes of `layer` from index `first` on (what the editor drew): see the
/// module docs.
pub(crate) fn punch(ctx: &egui::Context, layer: LayerId, first: usize, hole: Rect) {
    ctx.graphics_mut(|graphics| {
        let list = graphics.entry(layer);
        let end = list.next_idx().0;
        let mut more = Vec::new();
        for index in first..end {
            list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
                let mut parts = around(clipped.clip_rect, hole).into_iter();
                // The first part in its place, the others after: the same shape, clipped again.
                clipped.clip_rect = parts.next().unwrap_or(Rect::NOTHING);
                more.extend(parts.map(|part| (part, clipped.shape.clone())));
            });
        }
        for (clip, shape) in more {
            list.add(clip, shape);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: std::ops::RangeInclusive<f32>, y: std::ops::RangeInclusive<f32>) -> Rect {
        Rect::from_x_y_ranges(x, y)
    }

    /// Across: above and below; inside: around it on all four sides; apart: as it was.
    #[test]
    fn the_parts_go_around_the_hole() {
        let clip = rect(0.0..=100.0, 0.0..=100.0);
        assert_eq!(
            around(clip, rect(0.0..=100.0, 40.0..=60.0)),
            [rect(0.0..=100.0, 0.0..=40.0), rect(0.0..=100.0, 60.0..=100.0)]
        );
        assert_eq!(around(clip, rect(20.0..=80.0, 40.0..=60.0)).len(), 4);
        assert_eq!(around(clip, rect(200.0..=300.0, 40.0..=60.0)), [clip]);
        assert!(around(clip, rect(-10.0..=110.0, -10.0..=110.0)).is_empty(), "all under the hole");
    }
}
