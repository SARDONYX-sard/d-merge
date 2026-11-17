//! The details panel beside the node graph, as a blueprint's or Blender's: every field of the open
//! object, those of the objects nested in it too, each with an input of its type
//! (see [`super::details_input`]).
//!
//! ```text
//! ┌ Details ──────────────────────────┐
//! │ ▼ variableInfos [1]               │  a list, or a field over lines: opens and closes
//! │   ▼ [0]                           │
//! │     ▼ role                        │
//! │         role     [ROLE_DEFAULT ▾] │  an enum
//! │         flags    [0 ▾]            │  flags
//! │       type       [VARIABLE_TYP ▾] │
//! │ startStateId     [ 0 ]            │  an integer, in its type's range
//! │ wildcardTransit… [#0099      ] →  │  a pointer: `→` goes to it
//! └───────────────────────────────────┘
//! ```

use nemesis_diff::{
    details::{DetailKind, DetailRow, Label},
    fields::Token,
    schema::TypeKind,
};

/// The share of a row's width its name takes.
const NAME_SHARE: f32 = 0.42;

/// What the panel asked for.
#[derive(Debug)]
pub(super) enum DetailOut {
    /// The value at `token` (of the object's text) is `text` now.
    Edit { at: usize, len: usize, text: String },
    /// Go to the object of this id.
    GoTo(String),
}

/// The details of `text`, the object `id`. `editable`: its values can be changed.
///
/// `open_all`: `Some(true)` expands every group this frame, `Some(false)` collapses every one.
pub(super) fn details_ui(
    ui: &mut egui::Ui,
    (id, text): (&str, &str),
    (editable, open_all): (bool, Option<bool>),
) -> Option<DetailOut> {
    let rows = nemesis_diff::details::object_details(text);
    let mut out = None;
    egui::ScrollArea::vertical().id_salt("graph_details_scroll").auto_shrink(false).show(ui, |ui| {
        ui.add_enabled_ui(editable, |ui| rows_ui(ui, (id, &rows), open_all, &mut out));
    });
    out
}

/// `rows`, each group a header that opens to the deeper rows after it.
fn rows_ui(
    ui: &mut egui::Ui,
    (id, rows): (&str, &[DetailRow<'_>]),
    open_all: Option<bool>,
    out: &mut Option<DetailOut>,
) {
    let mut at = 0;
    while at < rows.len() {
        let row = rows[at];
        let end = at + 1 + rows[at + 1..].iter().take_while(|deeper| deeper.depth > row.depth).count();
        match row.kind {
            DetailKind::Group { count } => {
                let header =
                    egui::CollapsingHeader::new(group_title(row.label, count)).id_salt((id, row.at)).open(open_all);
                header.show(ui, |ui| rows_ui(ui, (id, &rows[at + 1..end]), open_all, out));
            }
            DetailKind::Value { token, schema, .. } => {
                let kind = row.kind.value_type();
                if let Some(done) = value_row(ui, (id, row.label, row.at), (token, schema, kind)) {
                    *out = Some(done);
                }
            }
        }
        at = end;
    }
}

/// e.g. `variableInfos [3]`, `[0]`, `role`
fn group_title(label: Label<'_>, count: Option<&str>) -> String {
    match (label, count) {
        (Label::Field(name), Some(count)) => format!("{name} [{count}]"),
        (Label::Field(name), None) => name.to_owned(),
        (Label::Element(index), _) => format!("[{index}]"),
    }
}

/// A value: its name, its input, and `→` for a pointer.
fn value_row(
    ui: &mut egui::Ui,
    (id, label, at): (&str, Label<'_>, usize),
    (token, schema, kind): (Token<'_>, Option<&'static nemesis_diff::schema::FieldSchema>, Option<TypeKind>),
) -> Option<DetailOut> {
    let mut out = None;
    ui.horizontal(|ui| {
        let name = match label {
            Label::Field(name) => std::borrow::Cow::Borrowed(name),
            Label::Element(index) => std::borrow::Cow::Owned(format!("[{index}]")),
        };
        let width = ui.available_width() * NAME_SHARE;
        ui.add_sized([width, ui.spacing().interact_size.y], egui::Label::new(name.as_ref()).truncate())
            .on_hover_text(name.as_ref());

        let is_pointer = kind == Some(TypeKind::Pointer);
        let go = egui_widgets::shadcn::Button::new("").icon(egui_widgets::shadcn::Icon::ArrowRight).small();
        if is_pointer && token.text.starts_with('#') && ui.add(go).clicked() {
            out = Some(DetailOut::GoTo(token.text.to_owned()));
        }
        let input_id = ui.make_persistent_id((id, at, token.at));
        if let Some(text) = super::details_input::value_input(ui, input_id, (token.text, schema, kind)) {
            out = Some(DetailOut::Edit { at: token.at, len: token.text.len(), text });
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_are_titled_by_name_and_count() {
        assert_eq!(group_title(Label::Field("variableInfos"), Some("3")), "variableInfos [3]");
        assert_eq!(group_title(Label::Field("role"), None), "role");
        assert_eq!(group_title(Label::Element(0), None), "[0]");
    }
}
