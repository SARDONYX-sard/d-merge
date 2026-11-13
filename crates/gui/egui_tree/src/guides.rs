//! The lines joining the rows of a tree, as VS Code's indent guides:
//!
//! ```text
//! #0090
//! ├ #0091
//! │ └ #0094     `│` is level 0 going on past this row, `└` the row's own line at level 1
//! └ #0095
//! ```
//! A row keeps which levels' lines go on past it as bits ([`bit`]): set when the ancestor at that
//! level has more children below. What is on the way to the selected row can be drawn in the
//! selection's color (`active`), so its branch stands out in a big tree.

use std::ops::RangeInclusive;

/// The bit of tree level `level` in [`GuideRow::guides`]. Deeper than 64 levels has no lines.
#[inline]
#[must_use]
pub const fn bit(level: u16) -> u64 {
    if level < u64::BITS as u16 { 1 << level } else { 0 }
}

/// Does the line of tree level `level` go on past a row with these `guides`?
#[inline]
#[must_use]
pub const fn has(guides: u64, level: u16) -> bool {
    guides & bit(level) != 0
}

/// What [`paint`] needs of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuideRow {
    /// `0`: a root, which has no line.
    pub depth: u16,
    /// The last child of its parent: its own line ends here (`└`, not `├`).
    pub last_child: bool,
    /// Bit `n` ([`bit`]): the line of level `n` goes on past this row.
    pub guides: u64,
}

/// Paints the lines of `row` (row `index` of the list) where the next row starts (`ui.cursor()`).
///
/// Each level is `indent` wide, the row `height` high. `active[level]`: the rows whose line at that
/// level is on the way to the selected row, drawn in the selection's color.
pub fn paint(
    ui: &egui::Ui,
    row: GuideRow,
    index: usize,
    (height, indent): (f32, f32),
    active: &[Option<RangeInclusive<usize>>],
) {
    let Some(own_level) = row.depth.checked_sub(1) else { return };
    let normal = egui::Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.5));
    let lit = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
    let way = |level: u16| active.get(usize::from(level)).and_then(Option::as_ref);
    let is_lit = |level: u16| way(level).is_some_and(|rows| rows.contains(&index));

    let (left, top) = (ui.cursor().left(), ui.cursor().top());
    // Up to the next row, over the gap between the rows.
    let bottom = top + height + ui.spacing().item_spacing.y;
    let center = top + height / 2.0;
    let x_of = |level: u16| f32::from(level).mul_add(indent, left + indent / 2.0);
    let painter = ui.painter();

    for level in (0..own_level).filter(|&level| has(row.guides, level)) {
        painter.vline(x_of(level), top..=bottom, if is_lit(level) { lit } else { normal });
    }
    let own_x = x_of(own_level);
    painter.vline(own_x, top..=if row.last_child { center } else { bottom }, normal);
    painter.hline(own_x..=own_x + indent / 2.0, center, normal);
    if is_lit(own_level) {
        // The way ends at this row if it is the one on the way: only down to its own branch.
        let ends_here = way(own_level).is_some_and(|rows| *rows.end() == index);
        painter.vline(own_x, top..=if ends_here { center } else { bottom }, lit);
        if ends_here {
            painter.hline(own_x..=own_x + indent / 2.0, center, lit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bit_per_level() {
        let guides = bit(0) | bit(2);
        assert!(has(guides, 0) && !has(guides, 1) && has(guides, 2));
        assert_eq!(bit(64), 0, "too deep: no line");
    }
}
