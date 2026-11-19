//! How an object is written in the tree and the lists:
//!
//! ```text
//! #0106  hkbStateMachine  GetUpStateMachine  ← #0105 #0109      the row, cut with `…` to fit
//!        ^ class color    ^ name color        ^ weak: who points to it
//! ```
//! When cut, the whole label is on the hover, a part per line, the pointers wrapped.
//!
//! An object changed since the last commit has, as in VS Code's explorer and the editor's tabs, its
//! `A` / `M` at the right end and its id in the same color:
//!
//! ```text
//! ● #0106  hkbStateMachine  GetUpStateMachine                         M
//! ```

use crate::patch::{
    tree::{Graph, Node},
    workspace::history::{ChangeKind, ObjectChange},
};

/// The width kept at the right end of a row for its `A` / `M`.
const MARK_WIDTH: f32 = 14.0;

/// What changed since the last commit in the active behavior (`behavior`, its `meshes_path`), as the
/// Changes view lists it: the rows are marked by it.
#[derive(Debug, Clone, Copy)]
pub(in crate::patch::side_area) struct Uncommitted<'a> {
    pub(in crate::patch::side_area) behavior: &'a str,
    pub(in crate::patch::side_area) changes: &'a [ObjectChange],
}

impl Uncommitted<'_> {
    /// How object `id` changed since the last commit, if it did.
    pub(in crate::patch::side_area) fn kind(&self, id: &str) -> Option<ChangeKind> {
        let change = self.changes.iter().find(|change| change.behavior == self.behavior && change.id == id);
        change.map(|change| change.kind)
    }
}

/// The button of a row of `node`, and its `A` / `M` at the right end if it changed since the last
/// commit. `(selected, dimmed)`: it is selected; it opens under another parent.
pub(in crate::patch::side_area) fn object_button(
    ui: &mut egui::Ui,
    graph: &Graph,
    node: Node,
    (selected, dimmed): (bool, bool),
    uncommitted: Uncommitted<'_>,
) -> egui::Response {
    let change = uncommitted.kind(graph.id(node));
    // Cut with `…` to the width left, as VS Code's explorer: the list never scrolls sideways.
    let mark = if change.is_some() { MARK_WIDTH } else { 0.0 };
    let width = ui.spacing().button_padding.x.mul_add(-2.0, ui.available_width()) - mark;
    let fitted = egui_widgets::fit(ui, label(ui, graph, node, (dimmed, change)), width);

    let response = ui.add(egui::Button::selectable(selected, fitted.galley));
    if let Some(kind) = change {
        let color = crate::patch::ui::change_color(ui, kind);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(kind.mark()).monospace().color(color));
        });
    }
    egui_widgets::CutHover::new().show_if(ui, response, fitted.elided, |ui| whole_label(ui, graph, node))
}

/// The row's text, each part in its color, to find them in a long list. `change`: its id is in the
/// color of how it changed since the last commit.
fn label(
    ui: &egui::Ui,
    graph: &Graph,
    node: Node,
    (dimmed, change): (bool, Option<ChangeKind>),
) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let tone = |color: egui::Color32| if dimmed { color.gamma_multiply(0.5) } else { color };

    let mut job = egui::text::LayoutJob::default();
    for (at, (text, color)) in colored_parts(graph, node, ui.visuals()).into_iter().enumerate() {
        let color = match change {
            Some(kind) if at == 0 => crate::patch::ui::change_color(ui, kind),
            _ => color,
        };
        job.append(text, 0.0, egui::TextFormat::simple(font.clone(), tone(color)));
    }
    job
}

/// The hover of a cut row: a part per line, wrapped.
fn whole_label(ui: &mut egui::Ui, graph: &Graph, node: Node) {
    let visuals = ui.visuals().clone();
    for (text, color) in colored_parts(graph, node, &visuals) {
        let text = text.trim();
        if !text.is_empty() {
            ui.add(egui::Label::new(egui::RichText::new(text).monospace().color(color)).wrap());
        }
    }
}

/// `[(id, text), (class, class color), (name, name color), (pointers, weak)]`.
fn colored_parts<'a>(graph: &'a Graph, node: Node, visuals: &egui::Visuals) -> [(&'a str, egui::Color32); 4] {
    let (head, class, name, links) = graph.label_parts(node);
    [
        (head, visuals.text_color()),
        (class, egui_widgets::markdown::class_color(visuals)),
        (name, egui_widgets::markdown::name_color(visuals)),
        (links, visuals.weak_text_color()),
    ]
}
