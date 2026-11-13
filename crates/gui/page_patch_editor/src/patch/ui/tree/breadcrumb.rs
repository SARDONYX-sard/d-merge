//! `◎ … › BehaviorGraph › Root › GetUpModList`: where the selected object is in the tree. `◎`
//! scrolls to it; a name selects that object.

use gui_shared::i18n::{I18nKey, I18nMap};

use super::TreeEvent;
use crate::patch::tree::{Graph, TreeState};

/// How many names of the way are shown at most; the rest are on the hover of `…`.
const MAX_CRUMBS: usize = 6;

/// Draws it, if the selected object is the cursor's row (it is not in a closed branch).
pub(super) fn show(
    ui: &mut egui::Ui,
    graph: &Graph,
    state: &TreeState,
    selected: Option<&str>,
    i18n: &I18nMap,
) -> Option<TreeEvent> {
    let row = state.rows().get(state.cursor)?;
    if selected != Some(graph.id(row.node)) {
        return None;
    }

    // The rows from the root to the cursor's.
    let way = {
        let mut way = state.ancestors(state.cursor);
        way.push(state.cursor);
        way
    };
    let hidden = way.len().saturating_sub(MAX_CRUMBS);

    let mut event = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        if ui.small_button("◎").on_hover_text(i18n.t(I18nKey::NemesisLocateSelected)).clicked() {
            event = Some(TreeEvent::Locate);
        }

        if hidden > 0 {
            let names: Vec<&str> = way[..hidden].iter().map(|&row| graph.short_name(state.rows()[row].node)).collect();
            ui.weak("…").on_hover_text(names.join(" › "));
        }

        for (at, &row) in way[hidden..].iter().enumerate() {
            if at > 0 || hidden > 0 {
                ui.weak("›");
            }
            if crumb(ui, graph, state.rows()[row].node) {
                event = Some(TreeEvent::Select(row));
            }
        }
    });

    ui.separator();
    event
}

/// One name of the way, its whole label on hover. Returns whether it was clicked.
fn crumb(ui: &mut egui::Ui, graph: &Graph, node: crate::patch::tree::Node) -> bool {
    let text = egui::RichText::new(graph.short_name(node)).small();
    ui.add(egui::Button::new(text).frame(false)).on_hover_text(graph.label(node)).clicked()
}
