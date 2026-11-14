//! The editor's tab bar, as VS Code's: the objects open, then the diffs / blames
//! ([`super::revision_view`]), drawn with [`egui_widgets::EditorTab`] and its menu. The tabs
//! themselves are [`crate::patch::editor::tabs`].

use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{TabEvent, editor::tabs::EditorTabs, tree::Graph};

/// What the tabs show of the editor and the work, besides the tabs themselves.
pub(super) struct TabMarks<'a> {
    /// The editor has the keyboard: the shown tab's line is the accent color.
    pub(super) focused: bool,
    /// The work's problems: a tab of an object with some is red (errors) or yellow (warnings only).
    pub(in crate::patch) problems: Option<&'a [crate::patch::problems::ObjectProblems]>,
}

impl TabMarks<'_> {
    /// The worst problem of object `id` of `behavior`, if it has any.
    fn severity(&self, behavior: usize, id: &str) -> Option<egui_code_editor::Severity> {
        let group = self.problems?.iter().find(|group| group.behavior == behavior && group.id == id)?;
        egui_code_editor::problems::worst(&group.problems)
    }
}

/// `▔▔▔▔▔▔▔▔▔▔▔▔▔`
/// `│ Root     ✖ │ GetUpModList ● │`: VS Code's tabs ([`egui_widgets::EditorTab`]). See
/// [`crate::patch::editor::tabs`].
///
/// Only the name, to keep the tabs short: the hover has the whole label, and the tree follows the tab.
/// A click shows a tab, a double click pins it, a middle click or `✖` closes it. `●`: the object is
/// changed; italic: the preview tab; red / yellow: it has problems.
///
/// `views`: the diffs / blames open (see [`super::revision_view::Views`]), as tabs after the objects'.
/// With one shown, no object's tab is (`current` is `""`).
pub(super) fn tab_bar(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    tabs: &EditorTabs,
    (behavior, current): (usize, &str),
    graph: &Graph,
    marks: TabMarks<'_>,
    views: &[super::revision_view::ViewTab<'_>],
) -> Option<TabEvent> {
    let menu = egui_widgets::TabMenu::from_texts(gui_shared::i18n::tab_menu_texts(i18n));
    let mut event = None;

    egui::ScrollArea::horizontal().id_salt("nemesis_editor_tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0; // the tabs touch, with a line between them
            for tab in tabs.of(behavior) {
                let id = tab.id.as_str();
                let shown = {
                    let node = graph.node_of(id);
                    let name = tab_name(node.map_or("", |node| graph.short_name(node)), id);
                    let tone = marks.severity(behavior, id).map(|severity| severity.color(ui.visuals()));
                    egui_widgets::EditorTab::new(egui::RichText::new(name), id == current)
                        .focused(marks.focused)
                        .modified(node.is_some_and(|node| graph.is_changed(node)))
                        .preview(!tab.pinned)
                        .tone(tone)
                        .hover(node.map_or(id, |node| graph.label(node)))
                        .show(ui)
                };

                let to_event = |action| TabEvent::Object(action, id.to_owned());
                if let Some(picked) = shown.action(&menu, to_event, |ui| object_items(ui, i18n, id)) {
                    event = Some(picked);
                }
            }

            // The diffs and blames, read only: `GetUpModList (#2) ↔ GetUpModList (working) 🔒 ✖`.
            for view in views {
                let shown = egui_widgets::EditorTab::new(egui::RichText::new(view.title), view.shown)
                    .focused(marks.focused)
                    .read_only(true)
                    .preview(!view.pinned)
                    .hover(view.title)
                    .show(ui);

                let to_event = |action| TabEvent::View(action, view.at);
                if let Some(picked) = shown.action(&menu, to_event, |ui| object_items(ui, i18n, view.id)) {
                    event = Some(picked);
                }
            }
        });
    });
    event
}

/// `Copy id / Copy Path / Reveal in File Explorer` of object `id`, the end of a tab's menu.
fn object_items(ui: &mut egui::Ui, i18n: &I18nMap, id: &str) -> Option<TabEvent> {
    let mut picked = None;
    if ui.button(i18n.t(I18nKey::NemesisCopyId)).clicked() {
        ui.ctx().copy_text(id.to_owned());
        ui.close();
    }
    // As the explorer's menu: the file the object is in.
    let hover = i18n.t(I18nKey::NemesisObjectFileHover);
    let files = [
        (I18nKey::NemesisCopyPath, crate::patch::tool::list::FileAction::CopyPath),
        (super::object_list::REVEAL_LABEL, crate::patch::tool::list::FileAction::Reveal),
    ];
    for (label, action) in files {
        if ui.button(i18n.t(label)).on_hover_text(hover).clicked() {
            picked = Some(TabEvent::File(action, id.to_owned()));
            ui.close();
        }
    }
    picked
}

/// What a tab says: the object's name, or the last part of the id of what has none, such as a file of
/// the animation data.
///
/// e.g., `("GetUpModList", "#0106")` -> `GetUpModList`, `("", "AData~A/m$New.txt")` -> `m$New.txt`
fn tab_name<'a>(name: &'a str, id: &'a str) -> &'a str {
    if name.is_empty() { id.rsplit('/').next().unwrap_or(id) } else { name }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_says_the_name() {
        assert_eq!(tab_name("GetUpModList", "#0106"), "GetUpModList");
        assert_eq!(tab_name("", "#0106"), "#0106");
        assert_eq!(tab_name("", "AData~A/m$New.txt"), "m$New.txt");
    }
}
