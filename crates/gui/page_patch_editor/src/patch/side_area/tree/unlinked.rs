//! `⚠ Not linked (2)`: the new objects nothing points to, above the tree, so they are in view
//! whatever is open.

use std::rc::Rc;

use gui_shared::i18n::{I18nKey, I18nMap};

use super::label::object_button;
use crate::patch::tree::Graph;

/// How high the list gets before it scrolls.
const MAX_HEIGHT: f32 = 80.0;

/// Draws the list, if there is anything in it. A click selects that object.
pub(super) fn show(ui: &mut egui::Ui, graph: &Graph, selected: &mut Option<Rc<str>>, i18n: &I18nMap) {
    let unlinked = graph.unlinked();
    if unlinked.is_empty() {
        return;
    }

    let title = format!("⚠ {} ({})", i18n.t(I18nKey::NemesisNotLinked), unlinked.len());
    ui.colored_label(ui.visuals().warn_fg_color, title).on_hover_text(i18n.t(I18nKey::NemesisNotLinkedHover));

    egui::ScrollArea::vertical().id_salt("nemesis_unlinked").max_height(MAX_HEIGHT).show(ui, |ui| {
        for &node in unlinked {
            let id = graph.id(node);
            let is_selected = selected.as_deref() == Some(id);
            if object_button(ui, graph, node, (is_selected, false)).clicked() {
                *selected = Some(id.into());
            }
        }
    });

    ui.separator();
}
