//! The parents of the rows scrolled to, pinned over the top of the tree, as VS Code's sticky
//! scroll: without them, a long list of children shows nothing of what they belong to.

use super::{INDENT, TreeEvent, label::object_button};
use crate::patch::tree::{Graph, MAX_STICKY, TreeState};

/// Draws them over the top of `view` (what is seen of the list). `step`: a row's height with its
/// spacing. A click selects one.
pub(super) fn show(
    ui: &mut egui::Ui,
    graph: &Graph,
    state: &TreeState,
    view: egui::Rect,
    step: f32,
) -> Option<TreeEvent> {
    let rows = state.sticky_rows(step, MAX_STICKY);
    if rows.is_empty() {
        return None;
    }

    // A band over the top rows, as solid as the panel.
    let area = {
        #[expect(clippy::cast_precision_loss, reason = "a few rows")]
        let height = rows.len() as f32 * step;
        egui::Rect::from_min_size(view.min, egui::vec2(view.width(), height))
    };
    ui.painter().rect_filled(area, 0.0, ui.visuals().window_fill);
    ui.painter().hline(area.x_range(), area.bottom(), ui.visuals().widgets.noninteractive.bg_stroke);

    let mut event = None;
    let mut band = ui.new_child(egui::UiBuilder::new().max_rect(area));
    band.set_clip_rect(area);
    for row in rows {
        let line = state.rows()[row];
        band.horizontal(|ui| {
            ui.add_space(f32::from(line.depth).mul_add(INDENT, -state.scroll_x).max(0.0));
            if object_button(ui, graph, line.node, (false, false)).clicked() {
                event = Some(TreeEvent::Select(row));
            }
        });
    }
    event
}
