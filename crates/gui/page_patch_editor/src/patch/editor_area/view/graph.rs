//! The node graph of the open object (`Space g`): its neighborhood as a blueprint, in place of the
//! text, with each object's fields on its node. Both show the same work, so switching is instant
//! and each follows the other:
//!
//! ```text
//! editor -> graph   the field on the cursor's line is drawn strongly (and its edge)
//! graph  -> editor  a node: go to it (double click: edit it); a field's name: edit that line
//! graph  -> work    a value typed, a pin dropped on a node (or off one): the object's text changes,
//!                   as if typed (see `edited_text`)
//! ```

use egui_graph_view::{Card, Edge, GraphEdit, GraphSource, Row, Slot};
use egui_widgets::shadcn::Icon;
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::fields::{self, FieldValue, ObjectField, Token};

use crate::patch::{
    EditorResult,
    editor_area::EditorPane,
    tree::{self, Graph, Node},
    workspace::{Behavior, PANDORA_DOC},
};

/// The accent of a new object, and of an edited one: as the `+` and `●` of the tree.
const NEW_ACCENT: egui::Color32 = egui::Color32::from_rgb(87, 171, 90);
const EDITED_ACCENT: egui::Color32 = egui::Color32::from_rgb(210, 160, 60);

/// A behavior's objects as the graph view reads them. A row's key, as an edge's, is a byte offset
/// in the object's text: of a value, of a pointer, or of a field's line.
struct ObjectGraph<'a> {
    graph: &'a Graph,
    behavior: &'a Behavior,
}

impl<'a> ObjectGraph<'a> {
    /// `(id, text)` of `node`'s object. No text: one only a pointer mentions.
    fn object(&self, node: Node) -> (&'a str, &'a str) {
        let id = self.graph.id(node);
        (id, self.behavior.text(id).unwrap_or_default())
    }

    /// e.g. `hkbClipGenerator`
    fn class(&self, node: Node) -> &'a str {
        self.graph.label_parts(node).1.trim()
    }
}

impl GraphSource for ObjectGraph<'_> {
    type Node = Node;

    fn version(&self) -> u64 {
        self.graph.generation()
    }

    /// e.g. `#0097` at byte 418, in field `states` -> `Edge { to: #0097, label: "states", key: 418 }`
    fn edges(&self, node: Node) -> impl Iterator<Item = Edge<'_, Node>> {
        let (id, text) = self.object(node);
        tree::pointer_fields(text, id).into_iter().filter_map(|pointer| {
            let to = self.graph.mentioned(pointer.id)?;
            Some(Edge { to, label: pointer.field, key: pointer.at })
        })
    }

    fn parents(&self, node: Node) -> impl Iterator<Item = Node> {
        self.graph.parents(node).iter().copied()
    }

    /// e.g. title `GetUpModList`, subtitle `● #0106  hkbModifierList`
    fn card(&self, node: Node) -> Card<'_> {
        let (head, class, _, _) = self.graph.label_parts(node);
        let subtitle = self.graph.label(node)[..head.len() + class.len()].trim();
        let accent = if self.graph.is_new(node) {
            Some(NEW_ACCENT)
        } else if self.graph.is_changed(node) {
            Some(EDITED_ACCENT)
        } else {
            None
        };
        let exists = self.graph.exists(node);
        let editable = exists && !self.behavior.is_ref(self.graph.id(node));
        let header = super::graph_colors::of_class(class.trim());
        let fan_in = self.graph.parents(node).len();
        Card { title: self.graph.short_name(node), subtitle, header, accent, dimmed: !exists, editable, fan_in }
    }

    fn rows(&self, node: Node) -> impl Iterator<Item = Row<'_>> {
        let (_, text) = self.object(node);
        fields::object_fields(text).into_iter().flat_map(field_rows)
    }

    /// By the schema: e.g. a `generators` pin takes an `hkbGenerator`, not an `hkbModifier`.
    fn accepts(&self, node: Node, key: usize, to: Node) -> bool {
        let (_, text) = self.object(node);
        let fields = fields::object_fields(text);
        let Some((field, _)) = pin_at(&fields, key) else { return false };
        field.schema.is_none_or(|schema| nemesis_diff::schema::pointer_accepts(schema, self.class(to)))
    }
}

/// The rows of a field: a value, a pin, a pin per element and one to add, or what it holds.
///
/// e.g. `states` of 2 -> `states[0] ●`, `states[1] ●`, `states +●`
fn field_rows(field: ObjectField<'_>) -> Vec<Row<'_>> {
    let label = field.name;
    let color = field.schema.and_then(super::graph_colors::of_field);
    let row = |slot, key| Row { label, index: None, slot, color, key };
    match field.value {
        FieldValue::Inline(token) if field.holds_pointers() => {
            vec![row(Slot::Pin { connected: token.text.starts_with('#') }, token.at)]
        }
        FieldValue::Inline(token) => vec![row(Slot::Value(token.text), token.at)],
        FieldValue::Pointers(list) => {
            let connected = Slot::Pin { connected: true };
            let elements = list.elements.iter().enumerate();
            let mut rows: Vec<Row<'_>> =
                elements.map(|(i, element)| Row { index: Some(i), ..row(connected, element.at) }).collect();
            rows.push(row(Slot::AddPin, list.close_at));
            rows
        }
        FieldValue::Block(count) => vec![row(Slot::Fixed(count.unwrap_or("…")), field.at)],
    }
}

/// Where a pin of row `key` is in its field.
#[derive(Debug, Clone, Copy)]
enum Pin<'a> {
    /// `<hkparam name="x">#0096</hkparam>` (or `null`).
    Inline(Token<'a>),
    /// Element `index` of a list.
    Element(usize),
    /// After the last element of a list.
    Add,
}

/// The field with the pin of row `key`, and where the pin is in it.
fn pin_at<'f, 'a>(fields: &'f [ObjectField<'a>], key: usize) -> Option<(&'f ObjectField<'a>, Pin<'a>)> {
    fields.iter().find_map(|field| match &field.value {
        FieldValue::Inline(token) if token.at == key && field.holds_pointers() => Some((field, Pin::Inline(*token))),
        FieldValue::Pointers(list) if list.close_at == key => Some((field, Pin::Add)),
        FieldValue::Pointers(list) => {
            list.elements.iter().position(|element| element.at == key).map(|index| (field, Pin::Element(index)))
        }
        _ => None,
    })
}

/// An edit of one object, with the ids it needs (see [`GraphEdit`]).
#[derive(Debug, Clone, Copy)]
enum Edit<'a> {
    /// The value at `key` is typed.
    Value(usize, &'a str),
    /// The pin at `key` points to this id.
    Connect(usize, &'a str),
    Disconnect(usize),
}

/// The object's text after `edit`. `None`: it does not apply, e.g. a value with `<` in it.
///
/// e.g. `Connect` on `wildcardTransitions` (`null`) with `#0099` -> `...">#0099</hkparam>`;
/// `Disconnect` of `states[0]` -> that line gone and `numelements` one less.
fn edited_text(text: &str, edit: Edit<'_>) -> Option<String> {
    let fields = fields::object_fields(text);
    match edit {
        Edit::Value(key, value) => {
            let token = fields.iter().find_map(|field| match field.value {
                FieldValue::Inline(token) if token.at == key && !field.holds_pointers() => Some(token),
                _ => None,
            })?;
            (!value.contains('<')).then(|| fields::replace(text, token, value))
        }
        Edit::Connect(key, id) => match pin_at(&fields, key)? {
            (_, Pin::Inline(token)) => Some(fields::replace(text, token, id)),
            (field, Pin::Element(index)) => {
                let FieldValue::Pointers(list) = &field.value else { return None };
                Some(fields::replace(text, list.elements[index], id))
            }
            (field, Pin::Add) => {
                let FieldValue::Pointers(list) = &field.value else { return None };
                Some(fields::append_pointer(text, list, id))
            }
        },
        Edit::Disconnect(key) => match pin_at(&fields, key)? {
            (_, Pin::Inline(token)) => Some(fields::replace(text, token, "null")),
            (field, Pin::Element(index)) => {
                let FieldValue::Pointers(list) = &field.value else { return None };
                Some(fields::remove_pointer(text, list, index))
            }
            (_, Pin::Add) => None,
        },
    }
}

/// The 0-based line of byte `at` of `text`.
fn line_of(text: &str, at: usize) -> usize {
    text.as_bytes()[..at.min(text.len())].iter().filter(|&&byte| byte == b'\n').count()
}

/// Can `id` be shown as a graph? Not the animation data's lines, nor the Pandora document.
pub(super) fn can_show(behavior: &Behavior, graph: &Graph, id: &str) -> bool {
    !behavior.is_anim_data() && id != PANDORA_DOC && graph.node_of(id).is_some()
}

impl EditorPane {
    /// Draws the graph of `id` in place of the editor. Where a click goes is `result.definition`,
    /// as the editor's "go to definition"; an edit is `result.graph_edit`.
    pub(super) fn graph_ui(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (behavior, graph): (&Behavior, &Graph),
        id: &str,
        result: &mut EditorResult,
    ) {
        let Some(focus) = graph.node_of(id) else { return };
        let source = ObjectGraph { graph, behavior };
        let (_, text) = source.object(focus);
        let details = Details { id, text, title: object_title(graph, focus), editable: source.card(focus).editable };
        details_panel(ui, i18n, details, &mut self.details_folded, result);
        let highlight = source.rows(focus).map(|row| row.key).find(|&key| line_of(text, key) == self.cursor_line);
        let output = self.graph_view.show(ui, &source, (focus, highlight), &graph_labels(i18n));

        if let Some(edit) = output.edit {
            result.graph_edit = graph_edit(&source, &edit);
        }
        let (go_to, edit) = if let Some((node, key)) = output.row_clicked {
            (Some((node, line_of(source.object(node).1, key))), true)
        } else if let Some(node) = output.double_clicked.filter(|&node| graph.exists(node)) {
            (Some((node, 0)), true)
        } else {
            let clicked = output.clicked.filter(|&node| node != focus && graph.exists(node));
            (clicked.map(|node| (node, 0)), false)
        };
        let Some((node, line)) = go_to else { return };

        result.definition = Some((graph.id(node).to_owned(), line));
        self.cursor_line = line; // the row drawn strongly is of the new place, before the editor shows it
        if edit {
            self.show_graph = false;
        }
    }

    /// The graph shows, or the text again: the keyboard goes back to the editor then.
    pub(super) const fn toggle_graph(&mut self) {
        self.show_graph = !self.show_graph;
        if !self.show_graph {
            self.focus = Some(crate::patch::FocusAt::Keep);
        }
    }
}

/// The hover texts of the graph's controls.
fn graph_labels(i18n: &I18nMap) -> egui_graph_view::GraphLabels<'_> {
    egui_graph_view::GraphLabels {
        zoom_out: i18n.t(I18nKey::GraphZoomOutHover),
        zoom_in: i18n.t(I18nKey::GraphZoomInHover),
        zoom_reset: i18n.t(I18nKey::GraphZoomResetHover),
        center: i18n.t(I18nKey::GraphCenterHover),
        fit: i18n.t(I18nKey::GraphFitHover),
        depth: i18n.t(I18nKey::GraphDepthHover),
    }
}

/// A small button of `icon` alone.
fn icon_button(ui: &mut egui::Ui, icon: Icon) -> egui::Response {
    ui.add(egui_widgets::shadcn::Button::new("").icon(icon).small())
}

/// What the details panel shows: the open object.
struct Details<'a> {
    id: &'a str,
    text: &'a str,
    /// See [`object_title`].
    title: &'a str,
    editable: bool,
}

/// `node`'s object as the tree names it, without who points to it.
///
/// e.g. `● #0106  hkbModifierList  GetUpModList`
fn object_title(graph: &Graph, node: Node) -> &str {
    let (head, class, name, _) = graph.label_parts(node);
    graph.label(node)[..head.len() + class.len() + name.len()].trim()
}

/// The details of the open object, right of the graph: folded to a strip with `»`, back with `«`. What it asks for goes to `result`.
fn details_panel(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    details: Details<'_>,
    folded: &mut bool,
    result: &mut EditorResult,
) {
    if *folded {
        let strip = egui::Panel::right("graph_details_folded").resizable(false).exact_size(28.0);
        strip.show(ui, |ui| {
            if icon_button(ui, Icon::ChevronLeft).on_hover_text(i18n.t(I18nKey::GraphDetailsShowHover)).clicked() {
                *folded = false;
            }
        });
        return;
    }

    let Details { id, text, title, editable } = details;
    let mut open_all = None;
    let panel = egui::Panel::right("graph_details").resizable(true).default_size(340.0);
    // Kept to the panel's width: see `egui_widgets::contained`.
    let out = panel
        .show(ui, |ui| {
            egui_widgets::contained(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(i18n.t(I18nKey::GraphDetailsLabel));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, Icon::ChevronRight)
                            .on_hover_text(i18n.t(I18nKey::GraphDetailsHideHover))
                            .clicked()
                        {
                            *folded = true;
                        }
                        ui.separator();
                        let collapse = icon_button(ui, Icon::ChevronsDownUp);
                        if collapse.on_hover_text(i18n.t(I18nKey::GraphDetailsCollapseHover)).clicked() {
                            open_all = Some(false);
                        }
                        if icon_button(ui, Icon::ChevronsUpDown)
                            .on_hover_text(i18n.t(I18nKey::GraphDetailsExpandHover))
                            .clicked()
                        {
                            open_all = Some(true);
                        }
                    });
                });
                // What it is of: the object open.
                ui.add(egui::Label::new(egui::RichText::new(title).monospace().strong()).truncate())
                    .on_hover_text(title);
                ui.separator();
                super::details::details_ui(ui, (id, text), (editable, open_all))
            })
        })
        .inner;
    match out {
        Some(super::details::DetailOut::Edit { at, len, text: value }) => {
            let token = Token { at, text: &text[at..at + len] };
            result.graph_edit = Some((id.to_owned(), fields::replace(text, token, &value)));
        }
        Some(super::details::DetailOut::GoTo(target)) => result.definition = Some((target, 0)),
        None => {}
    }
}

/// `(object, its new text)` of `edit`, if it applies.
fn graph_edit(source: &ObjectGraph<'_>, edit: &GraphEdit<Node>) -> Option<(String, String)> {
    let (node, edit) = match edit {
        GraphEdit::Value { node, key, text } => (*node, Edit::Value(*key, text)),
        GraphEdit::Connect { node, key, to } => (*node, Edit::Connect(*key, source.graph.id(*to))),
        GraphEdit::Disconnect { node, key } => (*node, Edit::Disconnect(*key)),
    };
    let (id, text) = source.object(node);
    Some((id.to_owned(), edited_text(text, edit)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MACHINE: &str = "<hkobject name=\"#0095\" class=\"hkbStateMachine\">
\t<hkparam name=\"startStateId\">0</hkparam>
\t<hkparam name=\"states\" numelements=\"1\">
\t\t#0097
\t</hkparam>
\t<hkparam name=\"wildcardTransitions\">null</hkparam>
</hkobject>
";

    fn key_of(rows: &[Row<'_>], label: &str, slot: impl Fn(&Slot<'_>) -> bool) -> usize {
        rows.iter().find(|row| row.label == label && slot(&row.slot)).map(|row| row.key).unwrap()
    }

    fn rows() -> Vec<Row<'static>> {
        fields::object_fields(MACHINE).into_iter().flat_map(field_rows).collect()
    }

    #[test]
    fn each_field_is_a_row() {
        let rows = rows();
        let slots: Vec<(&str, Option<usize>, Slot<'_>)> =
            rows.iter().map(|row| (row.label, row.index, row.slot)).collect();
        assert_eq!(
            slots,
            [
                ("startStateId", None, Slot::Value("0")),
                ("states", Some(0), Slot::Pin { connected: true }),
                ("states", None, Slot::AddPin),
                ("wildcardTransitions", None, Slot::Pin { connected: false }),
            ]
        );
    }

    #[test]
    fn a_value_is_typed_and_pins_connect() {
        let rows = rows();
        let value = key_of(&rows, "startStateId", |_| true);
        let typed = edited_text(MACHINE, Edit::Value(value, "2")).unwrap();
        assert!(typed.contains("<hkparam name=\"startStateId\">2</hkparam>"));
        assert_eq!(edited_text(MACHINE, Edit::Value(value, "<oops")), None);

        let wildcard = key_of(&rows, "wildcardTransitions", |_| true);
        let connected = edited_text(MACHINE, Edit::Connect(wildcard, "#0099")).unwrap();
        assert!(connected.contains("\"wildcardTransitions\">#0099</hkparam>"));
        let back = key_of(&rows, "wildcardTransitions", |_| true);
        assert_eq!(edited_text(&connected, Edit::Disconnect(back)).as_deref(), Some(MACHINE));

        let add = key_of(&rows, "states", |slot| *slot == Slot::AddPin);
        let added = edited_text(MACHINE, Edit::Connect(add, "#m$0")).unwrap();
        assert!(added.contains("numelements=\"2\">\n\t\t#0097\n\t\t#m$0\n\t</hkparam>"), "{added}");

        let first = key_of(&rows, "states", |slot| *slot == Slot::Pin { connected: true });
        let removed = edited_text(MACHINE, Edit::Disconnect(first)).unwrap();
        assert!(removed.contains("numelements=\"0\">\n\t</hkparam>"), "{removed}");
    }

    #[test]
    fn lines_are_counted_to_a_byte() {
        assert_eq!(line_of("a\nb\nc", 0), 0);
        assert_eq!(line_of("a\nb\nc", 2), 1);
        assert_eq!(line_of("a\nb\nc", 99), 2);
    }
}
