//! The flat list of objects: what the filter / "unlinked only" leaves, one row per object.
//!
//! ```text
//! #0106  hkbStateMachine  GetUpStateMachine     a click selects, a double click keeps it open
//! #0109  hkbClipGenerator Idle                  its right-click menu: see [`super::row_menu`]
//! ```
//! It sees only the rows, the graph and the selection; what a click asks for is returned.

use gui_shared::i18n::I18nMap;

use super::{RowAction, row_menu};
use crate::patch::{
    side_area::{
        explorer::{ListScroll, fixed_row, offset_to_show, row_height},
        tree::object_button,
    },
    tree::{Graph, Node},
};

/// What a row asked for.
#[derive(Debug, Clone, Copy)]
pub(super) enum FlatEvent {
    Select(Node),
    /// Its menu, or a double click (`KeepOpen`).
    Menu(Node, RowAction),
}

/// Draws the rows seen. `scroll_to`: a row to bring into view. Returns what was asked for, and where
/// the list is scrolled now.
pub(super) fn show(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    (graph, rows): (&Graph, &[Node]),
    selected: Option<&str>,
    (scroll_to, scroll): (Option<usize>, ListScroll),
) -> (Option<FlatEvent>, ListScroll) {
    let height = row_height(ui);
    // Up and down only, as VS Code's: the rows are cut with `…`. (Sideways, its width changed with the
    // rows drawn, and the scroll bars came and went.)
    let mut area = egui::ScrollArea::vertical().auto_shrink(false);
    let sizes = (height, ui.spacing().item_spacing.y);
    if let Some(offset) = scroll_to.and_then(|at| offset_to_show(at, sizes, rows.len(), scroll, 0)) {
        area = area.vertical_scroll_offset(offset);
    }

    let mut event = None;
    let output = area.show_rows(ui, height, rows.len(), |ui, range| {
        for &node in &rows[range] {
            let is_selected = selected == Some(graph.id(node));
            if let Some(asked) = row_ui(ui, i18n, graph, node, (height, is_selected)) {
                event = Some(asked);
            }
        }
    });

    let mut now = scroll;
    now.note(&output);
    (event, now)
}

/// One row: the object, and its menu.
fn row_ui(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    graph: &Graph,
    node: Node,
    (height, selected): (f32, bool),
) -> Option<FlatEvent> {
    let response = fixed_row(ui, height, |ui| object_button(ui, graph, node, (selected, false)));

    // A double click keeps it open; a click selects it.
    let mut event = if response.double_clicked() {
        Some(FlatEvent::Menu(node, RowAction::KeepOpen))
    } else {
        response.clicked().then_some(FlatEvent::Select(node))
    };

    response.context_menu(|ui| {
        if let Some(action) = row_menu(ui, i18n, graph.is_new(node), graph.is_changed(node)) {
            event = Some(FlatEvent::Menu(node, action));
        }
    });
    event
}
