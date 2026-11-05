//! Draws the object tree and reads its keys. The logic is in [`crate::patch::tree`].

use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::tree::{Graph, Node, Row, TreeInput, TreeState, has_guide};

/// Pixels of indent per tree level.
const INDENT: f32 = 12.0;

/// This frame's keys as [`TreeInput`]s (several for a held key on a slow frame).
///
/// `leader`: `Space` was pressed; the next key decides. Frames also come without keys (e.g., the
/// mouse moved), so only another key ends it. Kept by the caller between frames.
pub(in crate::patch) fn read_keys(ui: &egui::Ui, leader: &mut bool) -> Vec<TreeInput> {
    use egui::{Key, Modifiers};
    const BINDINGS: [(Key, TreeInput); 10] = [
        (Key::J, TreeInput::Next),
        (Key::ArrowDown, TreeInput::Next),
        (Key::K, TreeInput::Previous),
        (Key::ArrowUp, TreeInput::Previous),
        (Key::H, TreeInput::Close),
        (Key::ArrowLeft, TreeInput::Close),
        (Key::L, TreeInput::Open),
        (Key::ArrowRight, TreeInput::Open),
        (Key::Enter, TreeInput::Leave),
        (Key::Escape, TreeInput::Leave),
    ];
    let mut inputs = Vec::new();
    ui.input_mut(|input| {
        *leader |= input.consume_key(Modifiers::NONE, Key::Space);
        if *leader {
            let leave = input.consume_key(Modifiers::NONE, Key::E);
            if leave {
                inputs.push(TreeInput::Leave);
            }
            *leader = !leave && !input.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. }));
        }
        // Before `g`: `consume_key` with no modifiers also takes the Shift variant.
        if input.consume_key(Modifiers::SHIFT, Key::G) {
            inputs.push(TreeInput::Last);
        }
        if input.consume_key(Modifiers::NONE, Key::G) {
            inputs.push(TreeInput::First);
        }
        for (key, tree_input) in BINDINGS {
            while input.consume_key(Modifiers::NONE, key) {
                inputs.push(tree_input);
            }
        }
    });
    inputs
}

/// What a click on a row asked for.
#[derive(Debug, Clone, Copy)]
enum RowEvent {
    /// The row with this index was clicked.
    Select(usize),
    SetOpen(Node, bool),
    SetOpenBelow(Node, bool),
}

/// Draws the tree; a click selects an object (`selected`).
///
/// `changed_only`: only the branches leading to edited / new objects.
/// `focused`: the keyboard is in the tree: the selected row gets a frame.
pub(in crate::patch) fn tree_ui(
    ui: &mut egui::Ui,
    graph: &Graph,
    selected: &mut Option<String>,
    state: &mut TreeState,
    (changed_only, focused): (bool, bool),
    i18n: &I18nMap,
) {
    state.prepare(graph, selected.as_deref(), changed_only);
    unlinked_section(ui, graph, selected, i18n);

    let row_height = ui.spacing().interact_size.y.max(ui.text_style_height(&egui::TextStyle::Monospace) + 4.0);
    let mut scroll = egui::ScrollArea::both().id_salt("nemesis_tree").auto_shrink(false);
    let row_with_spacing = row_height + ui.spacing().item_spacing.y;
    if let Some(offset) = state.scroll_to.take().and_then(|row| state.offset_for(row, row_with_spacing)) {
        scroll = scroll.vertical_scroll_offset(offset);
    }

    let mut event = None;
    let output = scroll.show_rows(ui, row_height, state.rows().len(), |ui, range| {
        for (i, row) in state.rows()[range.clone()].iter().enumerate() {
            let is_selected = selected.as_deref() == Some(graph.id(row.node));
            let row_event = row_ui(ui, graph, *row, row_height, (is_selected, focused), range.start + i, i18n);
            event = row_event.or(event);
        }
    });
    state.scroll_offset = output.state.clamped_offset().y;
    state.view_height = output.inner_rect.height();

    match event {
        Some(RowEvent::Select(row)) => {
            state.cursor = row;
            let id = graph.id(state.rows()[row].node);
            id.clone_into(selected.get_or_insert_default());
        }
        Some(RowEvent::SetOpen(node, open)) => state.set_open(graph, node, open),
        Some(RowEvent::SetOpenBelow(node, open)) => state.set_open_below(graph, node, open),
        None => {}
    }
}

/// `⚠ Not linked (2)` and the new objects nothing points to. Above the tree: always in view,
/// whatever is expanded.
fn unlinked_section(ui: &mut egui::Ui, graph: &Graph, selected: &mut Option<String>, i18n: &I18nMap) {
    let unlinked = graph.unlinked();
    if unlinked.is_empty() {
        return;
    }
    let title = format!("⚠ {} ({})", i18n.t(I18nKey::NemesisNotLinked), unlinked.len());
    ui.colored_label(ui.visuals().warn_fg_color, title).on_hover_text(i18n.t(I18nKey::NemesisNotLinkedHover));
    egui::ScrollArea::vertical().id_salt("nemesis_unlinked").max_height(80.0).show(ui, |ui| {
        for &node in unlinked {
            let id = graph.id(node);
            let text = object_label(ui, graph, node, false);
            if ui.add(egui::Button::selectable(selected.as_deref() == Some(id), text)).clicked() {
                id.clone_into(selected.get_or_insert_default());
            }
        }
    });
    ui.separator();
}

/// One row: `▶ label`, with the expand / collapse menu on nodes that have children.
fn row_ui(
    ui: &mut egui::Ui,
    graph: &Graph,
    row: Row,
    height: f32,
    (is_selected, focused): (bool, bool),
    index: usize,
    i18n: &I18nMap,
) -> Option<RowEvent> {
    let mut event = None;
    ui.horizontal(|ui| {
        ui.set_height(height);
        draw_connecting_lines(ui, row, height);
        ui.add_space(f32::from(row.depth) * INDENT);
        let glyph = if row.expandable && row.open {
            "▼"
        } else if row.expandable {
            "▶"
        } else {
            " "
        };
        if ui.add_enabled(row.expandable, egui::Button::new(glyph).frame(false)).clicked() {
            event = Some(RowEvent::SetOpen(row.node, !row.open));
        }

        let response = ui.add(egui::Button::selectable(is_selected, object_label(ui, graph, row.node, row.dimmed)));
        if response.clicked() {
            event = Some(RowEvent::Select(index));
        }
        if focused && is_selected {
            let stroke = ui.visuals().selection.stroke;
            ui.painter().rect_stroke(response.rect, 2.0, stroke, egui::StrokeKind::Outside);
        }
        if row.expandable {
            response.context_menu(|ui| {
                if ui.button(i18n.t(I18nKey::NemesisExpandAllBelow)).clicked() {
                    event = Some(RowEvent::SetOpenBelow(row.node, true));
                    ui.close();
                }
                if ui.button(i18n.t(I18nKey::NemesisCollapseAllBelow)).clicked() {
                    event = Some(RowEvent::SetOpenBelow(row.node, false));
                    ui.close();
                }
            });
        }
    });
    event
}

/// The lines joining a row to its parent and to the rows below:
/// ```text
/// #0090
/// ├ #0091
/// │ └ #0094     `│` is level 0 going on past this row, `└` the row's own line at level 1
/// └ #0095
/// ```
fn draw_connecting_lines(ui: &egui::Ui, row: Row, height: f32) {
    let Some(own_level) = row.depth.checked_sub(1) else { return };
    let stroke = egui::Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.5));
    let left = ui.cursor().left();
    let top = ui.cursor().top();
    // Up to the next row, over the gap between the rows.
    let bottom = top + height + ui.spacing().item_spacing.y;
    let center = top + height / 2.0;
    let x_of = |level: u16| f32::from(level).mul_add(INDENT, left + INDENT / 2.0);
    let painter = ui.painter();

    for level in 0..own_level {
        if has_guide(row.guides, level) {
            painter.vline(x_of(level), top..=bottom, stroke);
        }
    }
    let own_x = x_of(own_level);
    painter.vline(own_x, top..=if row.last_child { center } else { bottom }, stroke);
    painter.hline(own_x..=own_x + INDENT / 2.0, center, stroke);
}

/// A row's text: the class and the name each have their color, to find them in a long list.
/// `dimmed`: the object is expanded at another place.
pub(in crate::patch) fn object_label(ui: &egui::Ui, graph: &Graph, node: Node, dimmed: bool) -> egui::text::LayoutJob {
    let (head, class, name, links) = graph.label_parts(node);
    let visuals = ui.visuals();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let (class_color, name_color) = if visuals.dark_mode {
        (egui::Color32::from_rgb(86, 182, 194), egui::Color32::from_rgb(229, 192, 123))
    } else {
        (egui::Color32::from_rgb(0, 110, 130), egui::Color32::from_rgb(150, 90, 0))
    };
    let tone = |color: egui::Color32| if dimmed { color.gamma_multiply(0.5) } else { color };

    let mut job = egui::text::LayoutJob::default();
    let parts =
        [(head, visuals.text_color()), (class, class_color), (name, name_color), (links, visuals.weak_text_color())];
    for (text, color) in parts {
        job.append(text, 0.0, egui::TextFormat::simple(font.clone(), tone(color)));
    }
    job
}
