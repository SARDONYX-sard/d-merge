//! How an object is written in the tree and the lists:
//!
//! ```text
//! #0106  hkbStateMachine  GetUpStateMachine  ← #0105 #0109      the row, cut with `…` to fit
//!        ^ class color    ^ name color        ^ weak: who points to it
//! ```
//! When cut, the whole label is on the hover, a part per line, the pointers wrapped.

use crate::patch::tree::{Graph, Node};

/// The button of a row of `node`. `(selected, dimmed)`: it is selected; it opens under another
/// parent.
pub(in crate::patch::side_area) fn object_button(
    ui: &mut egui::Ui,
    graph: &Graph,
    node: Node,
    (selected, dimmed): (bool, bool),
) -> egui::Response {
    // Cut with `…` to the width left, as VS Code's explorer: the list never scrolls sideways.
    let width = ui.spacing().button_padding.x.mul_add(-2.0, ui.available_width());
    let fitted = egui_widgets::fit(ui, label(ui, graph, node, dimmed), width);

    let response = ui.add(egui::Button::selectable(selected, fitted.galley));
    egui_widgets::CutHover::new().show_if(ui, response, fitted.elided, |ui| whole_label(ui, graph, node))
}

/// The row's text, each part in its color, to find them in a long list.
fn label(ui: &egui::Ui, graph: &Graph, node: Node, dimmed: bool) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let tone = |color: egui::Color32| if dimmed { color.gamma_multiply(0.5) } else { color };

    let mut job = egui::text::LayoutJob::default();
    for (text, color) in colored_parts(graph, node, ui.visuals()) {
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
