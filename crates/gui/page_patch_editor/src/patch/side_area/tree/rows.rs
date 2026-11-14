//! The rows of the tree: their lines, `▶` / `▼`, the object, and its right-click menu.
//!
//! ```text
//! │ ├ ▶ #0094 hkbBehaviorGraphData        a click selects, a double click keeps it open
//! │ └ ▼ #0095 hkbStateMachine  Root       `▶` / `▼` opens / closes
//! │     #0097 hkbClipGenerator            dimmed: open under another parent
//! ```

use gui_shared::i18n::{I18nKey, I18nMap};

use super::{INDENT, TreeEvent, label::object_button};
use crate::patch::{
    side_area::{
        explorer::{fixed_row, row_height},
        object_list::{RowAction, row_menu},
    },
    tree::{Graph, MAX_STICKY, Row, TreeState},
};

/// What [`show`] drew.
pub(super) struct Shown {
    pub(super) event: Option<TreeEvent>,
    /// What is seen of the list, for the rows pinned over it.
    pub(super) view: egui::Rect,
    /// A row's height with the spacing after it.
    pub(super) step: f32,
}

/// Draws the rows seen (up and down only, as VS Code's explorer: a row is cut with `…`).
///
/// `(selected, focused)`: the selected object; the keyboard is in the tree.
pub(super) fn show(
    ui: &mut egui::Ui,
    graph: &Graph,
    state: &mut TreeState,
    (selected, focused): (Option<&str>, bool),
    i18n: &I18nMap,
) -> Shown {
    let height = row_height(ui);
    let spacing = ui.spacing().item_spacing.y;
    let scroll = egui::ScrollArea::vertical().id_salt("nemesis_tree").auto_shrink(false);
    let scroll = scrolled(scroll, state, (height, spacing));

    let mut event = None;
    let guides = state.active_guides();
    let output = scroll.show_rows(ui, height, state.rows().len(), |ui, range| {
        for index in range {
            let row = state.rows()[index];
            let is_selected = selected == Some(graph.id(row.node));

            egui_tree::guides::paint(ui, row.lines(), index, (height, INDENT), &guides);
            if let Some(clicked) = row_ui(ui, graph, (row, index), height, (is_selected, focused), i18n) {
                event = Some(clicked);
            }
        }
    });

    state.scroll.note(&output);
    state.scroll_x = output.state.clamped_offset().x;
    state.view_width = output.inner_rect.width();

    Shown { event, view: output.inner_rect, step: height + spacing }
}

/// `scroll`, moved to the row asked for: below its pinned parents, and sideways to its indent.
fn scrolled(mut scroll: egui::ScrollArea, state: &mut TreeState, sizes: (f32, f32)) -> egui::ScrollArea {
    let Some(row) = state.scroll_to.take() else { return scroll };

    if let Some(offset) = state.offset_for(row, sizes, MAX_STICKY) {
        scroll = scroll.vertical_scroll_offset(offset);
    }

    let indent = state.rows().get(row).map_or(0.0, |row| f32::from(row.depth) * INDENT);
    if let Some(offset) = state.offset_x_for(indent) {
        scroll = scroll.horizontal_scroll_offset(offset);
    }
    scroll
}

/// One row: its indent, `▶` / `▼`, and the object. Returns what a click on it asked for.
fn row_ui(
    ui: &mut egui::Ui,
    graph: &Graph,
    (row, index): (Row, usize),
    height: f32,
    (is_selected, focused): (bool, bool),
    i18n: &I18nMap,
) -> Option<TreeEvent> {
    let mut event = None;
    fixed_row(ui, height, |ui| {
        ui.add_space(f32::from(row.depth) * INDENT);

        if ui.add_enabled(row.expandable, egui::Button::new(glyph(row)).frame(false)).clicked() {
            event = Some(TreeEvent::SetOpen(row.node, !row.open));
        }

        let response = object_button(ui, graph, row.node, (is_selected, row.dimmed));
        if response.clicked() {
            event = Some(TreeEvent::Select(index));
        }
        if response.double_clicked() {
            event = Some(TreeEvent::Menu(index, RowAction::KeepOpen));
        }
        if focused && is_selected {
            egui_tree::list::cursor_frame(ui, response.rect);
        }

        response.context_menu(|ui| {
            if let Some(picked) = menu(ui, graph, (row, index), i18n) {
                event = Some(picked);
                ui.close();
            }
        });
    });
    event
}

/// `▼` open, `▶` closed, ` ` nothing to open.
const fn glyph(row: Row) -> &'static str {
    match (row.expandable, row.open) {
        (true, true) => "▼",
        (true, false) => "▶",
        (false, _) => " ",
    }
}

/// A row's right-click menu: the same as the flat list's, then what only a tree has.
fn menu(ui: &mut egui::Ui, graph: &Graph, (row, index): (Row, usize), i18n: &I18nMap) -> Option<TreeEvent> {
    if let Some(picked) = row_menu(ui, i18n, graph.is_new(row.node), graph.is_changed(row.node)) {
        return Some(TreeEvent::Menu(index, picked));
    }
    if !row.expandable {
        return None;
    }

    ui.separator();
    let below = [(I18nKey::NemesisExpandAllBelow, true), (I18nKey::NemesisCollapseAllBelow, false)];
    let mut picked = None;
    for (label, open) in below {
        if ui.button(i18n.t(label)).clicked() {
            picked = Some(TreeEvent::SetOpenBelow(row.node, open));
        }
    }
    picked
}
