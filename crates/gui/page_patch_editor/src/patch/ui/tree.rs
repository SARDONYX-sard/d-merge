//! Draws the object tree and reads its keys. The logic is in [`crate::patch::tree`].

use std::rc::Rc;

use gui_shared::i18n::{I18nKey, I18nMap};

use super::object_list::{RowAction, row_menu};
use crate::patch::{
    tool::list::{fixed_row, row_height},
    tree::{Graph, MAX_STICKY, Node, Row, TreeInput, TreeState, has_guide},
};

/// Pixels of indent per tree level.
const INDENT: f32 = 12.0;

/// What the explorer shows, switched by a key while it has the keyboard, as Vim file trees do.
///
/// | key | switches            |
/// | --- | ------------------- |
/// | `t` | tree / flat list    |
/// | `c` | changed only        |
/// | `u` | unlinked only       |
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(in crate::patch) struct ViewKeys {
    pub(in crate::patch) tree: bool,
    pub(in crate::patch) changed: bool,
    pub(in crate::patch) unlinked: bool,
}

/// This frame's [`ViewKeys`]: read before [`read_keys`], whichever of the tree and the list shows.
pub(in crate::patch) fn read_view_keys(ui: &egui::Ui) -> ViewKeys {
    use egui::{Key, Modifiers};
    ui.input_mut(|input| ViewKeys {
        tree: input.consume_key(Modifiers::NONE, Key::T),
        changed: input.consume_key(Modifiers::NONE, Key::C),
        unlinked: input.consume_key(Modifiers::NONE, Key::U),
    })
}

/// This frame's keys as [`TreeInput`]s (several for a held key on a slow frame).
///
/// `leader`: `Space` was pressed; the next key decides. Frames also come without keys (e.g., the
/// mouse moved), so only another key ends it. Kept by the caller between frames.
pub(in crate::patch) fn read_keys(ui: &egui::Ui, leader: &mut bool) -> Vec<TreeInput> {
    use egui::{Key, Modifiers};
    const BINDINGS: [(Key, TreeInput); 11] = [
        (Key::J, TreeInput::Next),
        (Key::ArrowDown, TreeInput::Next),
        (Key::K, TreeInput::Previous),
        (Key::ArrowUp, TreeInput::Previous),
        (Key::H, TreeInput::Close),
        (Key::ArrowLeft, TreeInput::Close),
        (Key::L, TreeInput::Open),
        (Key::ArrowRight, TreeInput::Open),
        (Key::O, TreeInput::Toggle),
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
    /// Bring the selected row into view.
    Locate,
    /// Double click (`KeepOpen`) or the menu of the row with this index: see [`RowAction`].
    Menu(usize, RowAction),
}

/// Draws the tree; a click selects an object (`selected`).
///
/// `changed_only`: only the branches leading to edited / new objects.
/// `focused`: the keyboard is in the tree: the selected row gets a frame.
///
/// Returns what a row's menu (or a double click: keep it open) asked for, of the row's object.
pub(super) fn tree_ui(
    ui: &mut egui::Ui,
    graph: &Graph,
    selected: &mut Option<Rc<str>>,
    state: &mut TreeState,
    (changed_only, focused): (bool, bool),
    i18n: &I18nMap,
) -> Option<(Node, RowAction)> {
    state.prepare(graph, selected.as_deref(), changed_only);
    unlinked_section(ui, graph, selected, i18n);
    let mut event = breadcrumb(ui, graph, state, selected.as_deref(), i18n);

    let row_height = row_height(ui);
    let mut scroll = egui::ScrollArea::both().id_salt("nemesis_tree").auto_shrink(false);
    let row_with_spacing = row_height + ui.spacing().item_spacing.y;
    if let Some(row) = state.scroll_to.take() {
        if let Some(offset) = state.offset_for(row, (row_height, ui.spacing().item_spacing.y)) {
            scroll = scroll.vertical_scroll_offset(offset);
        }
        let indent = state.rows().get(row).map_or(0.0, |row| f32::from(row.depth) * INDENT);
        if let Some(offset) = state.offset_x_for(indent) {
            scroll = scroll.horizontal_scroll_offset(offset);
        }
    }

    let guides = state.active_guides();
    let output = scroll.show_rows(ui, row_height, state.rows().len(), |ui, range| {
        for (i, row) in state.rows()[range.clone()].iter().enumerate() {
            let is_selected = selected.as_deref() == Some(graph.id(row.node));
            draw_connecting_lines(ui, *row, range.start + i, row_height, &guides);
            let row_event = row_ui(ui, graph, *row, row_height, (is_selected, focused), range.start + i, i18n);
            event = row_event.or(event);
        }
    });
    state.scroll_offset = output.state.clamped_offset().y;
    state.view_height = output.inner_rect.height();
    state.scroll_x = output.state.clamped_offset().x;
    state.view_width = output.inner_rect.width();
    event = sticky_rows_ui(ui, graph, state, output.inner_rect, row_with_spacing).or(event);

    let mut action = None;
    match event {
        Some(RowEvent::Menu(row, picked)) => {
            state.cursor = row;
            action = state.rows().get(row).map(|row| (row.node, picked));
        }
        Some(RowEvent::Select(row)) => {
            state.cursor = row;
            let id = graph.id(state.rows()[row].node);
            *selected = Some(id.into());
        }
        Some(RowEvent::SetOpen(node, open)) => state.set_open(graph, node, open),
        Some(RowEvent::SetOpenBelow(node, open)) => state.set_open_below(graph, node, open),
        Some(RowEvent::Locate) => state.scroll_to = Some(state.cursor),
        None => {}
    }
    action
}

/// `⚠ Not linked (2)` and the new objects nothing points to. Above the tree: always in view,
/// whatever is expanded.
fn unlinked_section(ui: &mut egui::Ui, graph: &Graph, selected: &mut Option<Rc<str>>, i18n: &I18nMap) {
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
                *selected = Some(id.into());
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
    fixed_row(ui, height, |ui| {
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
        if response.double_clicked() {
            event = Some(RowEvent::Menu(index, RowAction::KeepOpen));
        }
        if focused && is_selected {
            let stroke = ui.visuals().selection.stroke;
            ui.painter().rect_stroke(response.rect, 2.0, stroke, egui::StrokeKind::Outside);
        }
        response.context_menu(|ui| {
            // The same as the flat list's, then what only a tree has.
            if let Some(picked) = row_menu(ui, i18n, graph.is_new(row.node), graph.is_changed(row.node)) {
                event = Some(RowEvent::Menu(index, picked));
                ui.close();
            }
            if row.expandable {
                ui.separator();
                if ui.button(i18n.t(I18nKey::NemesisExpandAllBelow)).clicked() {
                    event = Some(RowEvent::SetOpenBelow(row.node, true));
                    ui.close();
                }
                if ui.button(i18n.t(I18nKey::NemesisCollapseAllBelow)).clicked() {
                    event = Some(RowEvent::SetOpenBelow(row.node, false));
                    ui.close();
                }
            }
        });
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
/// `guides`: what is on the way to the selected row is drawn in the selection's color, so its
/// branch stands out in a big tree. See [`TreeState::active_guides`].
fn draw_connecting_lines(
    ui: &egui::Ui,
    row: Row,
    index: usize,
    height: f32,
    guides: &[Option<std::ops::RangeInclusive<usize>>],
) {
    let Some(own_level) = row.depth.checked_sub(1) else { return };
    let normal = egui::Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.5));
    let active = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
    let is_active =
        |level: u16| guides.get(usize::from(level)).and_then(Option::as_ref).is_some_and(|r| r.contains(&index));

    let (left, top) = (ui.cursor().left(), ui.cursor().top());
    // Up to the next row, over the gap between the rows.
    let bottom = top + height + ui.spacing().item_spacing.y;
    let center = top + height / 2.0;
    let x_of = |level: u16| f32::from(level).mul_add(INDENT, left + INDENT / 2.0);
    let painter = ui.painter();

    for level in 0..own_level {
        if has_guide(row.guides, level) {
            painter.vline(x_of(level), top..=bottom, if is_active(level) { active } else { normal });
        }
    }
    let own_x = x_of(own_level);
    painter.vline(own_x, top..=if row.last_child { center } else { bottom }, normal);
    painter.hline(own_x..=own_x + INDENT / 2.0, center, normal);
    if is_active(own_level) {
        // The way ends at this row if it is the one on the way: only down to its own branch.
        let on_the_way_here = guides[usize::from(own_level)].as_ref().is_some_and(|r| *r.end() == index);
        painter.vline(own_x, top..=if on_the_way_here { center } else { bottom }, active);
        if on_the_way_here {
            painter.hline(own_x..=own_x + INDENT / 2.0, center, active);
        }
    }
}

/// How many crumbs of the way to the selected row are shown at most; the rest are on hover.
const MAX_CRUMBS: usize = 6;

/// `◎ … › BehaviorGraph › Root › GetUpModList`: where the selected object is. `◎` scrolls to it, a
/// name selects that object.
fn breadcrumb(
    ui: &mut egui::Ui,
    graph: &Graph,
    state: &TreeState,
    selected: Option<&str>,
    i18n: &I18nMap,
) -> Option<RowEvent> {
    let row = *state.rows().get(state.cursor)?;
    if selected != Some(graph.id(row.node)) {
        return None; // not in the tree as it is expanded now
    }
    let mut chain = state.ancestors(state.cursor);
    chain.push(state.cursor);
    let hidden = chain.len().saturating_sub(MAX_CRUMBS);

    let mut event = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        if ui.small_button("◎").on_hover_text(i18n.t(I18nKey::NemesisLocateSelected)).clicked() {
            event = Some(RowEvent::Locate);
        }
        if hidden > 0 {
            let names: Vec<&str> = chain[..hidden].iter().map(|&r| graph.short_name(state.rows()[r].node)).collect();
            ui.weak("…").on_hover_text(names.join(" › "));
        }
        for (i, &crumb) in chain[hidden..].iter().enumerate() {
            if i > 0 || hidden > 0 {
                ui.weak("›");
            }
            let node = state.rows()[crumb].node;
            let text = egui::RichText::new(graph.short_name(node)).small();
            if ui.add(egui::Button::new(text).frame(false)).on_hover_text(graph.label(node)).clicked() {
                event = Some(RowEvent::Select(crumb));
            }
        }
    });
    ui.separator();
    event
}

/// The ancestors of the rows on screen, pinned over the top of the tree. Without them, a long list
/// of children shows nothing of what they belong to. A click selects one.
fn sticky_rows_ui(
    ui: &mut egui::Ui,
    graph: &Graph,
    state: &TreeState,
    view: egui::Rect,
    step: f32,
) -> Option<RowEvent> {
    let rows = state.sticky_rows(step, MAX_STICKY);
    if rows.is_empty() {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "a few rows")]
    let height = rows.len() as f32 * step;
    let area = egui::Rect::from_min_size(view.min, egui::vec2(view.width(), height));
    ui.painter().rect_filled(area, 0.0, ui.visuals().window_fill);
    ui.painter().hline(area.x_range(), area.bottom(), ui.visuals().widgets.noninteractive.bg_stroke);

    let mut event = None;
    let mut sticky = ui.new_child(egui::UiBuilder::new().max_rect(area));
    sticky.set_clip_rect(area);
    for row in rows {
        let line = state.rows()[row];
        sticky.horizontal(|ui| {
            ui.add_space(f32::from(line.depth).mul_add(INDENT, -state.scroll_x).max(0.0));
            if ui.add(egui::Button::selectable(false, object_label(ui, graph, line.node, false))).clicked() {
                event = Some(RowEvent::Select(row));
            }
        });
    }
    event
}

/// A row's text: the class and the name each have their color, to find them in a long list.
/// `dimmed`: the object is expanded at another place.
pub(in crate::patch) fn object_label(ui: &egui::Ui, graph: &Graph, node: Node, dimmed: bool) -> egui::text::LayoutJob {
    let (head, class, name, links) = graph.label_parts(node);
    let visuals = ui.visuals();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let (class_color, name_color) =
        (egui_widgets::markdown::class_color(visuals), egui_widgets::markdown::name_color(visuals));
    let tone = |color: egui::Color32| if dimmed { color.gamma_multiply(0.5) } else { color };

    let mut job = egui::text::LayoutJob::default();
    let parts =
        [(head, visuals.text_color()), (class, class_color), (name, name_color), (links, visuals.weak_text_color())];
    for (text, color) in parts {
        job.append(text, 0.0, egui::TextFormat::simple(font.clone(), tone(color)));
    }
    job
}
