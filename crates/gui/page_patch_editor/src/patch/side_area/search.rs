//! The side bar's search: [`egui_code_editor::SearchView`] over the objects of the active behavior,
//! each found line under its object, named as the tree names it:
//!
//! ```text
//! ▼ #0106  hkbModifierList  GetUpModList      2
//!     <hkparam name="id">$eventID[moveStart]$</hkparam>
//! ```
//!
//! Replacing changes the behavior's objects, not those of the referenced mods.

use egui::text::LayoutJob;
use egui_code_editor::{DocumentEdit, SearchAsk, SearchLabels, SearchView, languages::Xml};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    side_area::{SideArea, SideAsk},
    tool::{selection::Selection, work_state::Work},
};

impl SideArea {
    /// See the module docs. A found line clicked is asked for.
    pub(super) fn search_panel(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        work: &mut Work,
        sel: &Selection,
    ) -> Option<SideAsk> {
        let generation = work.names(sel.active).graph.generation();
        let behavior = &work.workspace.behaviors[sel.active];
        self.search.refresh((sel.active, generation), behavior.texts().map(|object| (object.id, object.text)));

        let labels = SearchLabels {
            hint: i18n.t(I18nKey::SearchTextHint),
            regex_hover: i18n.t(I18nKey::SearchRegexHover),
            results_in: i18n.t(I18nKey::SearchResultsIn),
            documents: i18n.t(I18nKey::SearchObjects),
            replace_hint: i18n.t(I18nKey::SearchReplaceHint),
            replace_all_hover: i18n.t(I18nKey::SearchReplaceAllHover),
            replace_line_hover: i18n.t(I18nKey::SearchReplaceLineHover),
            toggle_replace_hover: i18n.t(I18nKey::SearchToggleReplaceHover),
        };
        // `#0106  hkbModifierList  GetUpModList`
        let label = {
            let font = egui::TextStyle::Monospace.resolve(ui.style());
            move |job: &mut LayoutJob, id: &str, visuals: &egui::Visuals| {
                let object = behavior.object_text(id);
                let (class, name) = object.map_or(("", ""), |object| (object.class, object.name));
                let parts = [
                    (id, visuals.strong_text_color()),
                    ("  ", visuals.text_color()),
                    (class, egui_widgets::markdown::class_color(visuals)),
                    ("  ", visuals.text_color()),
                    (name, visuals.weak_text_color()),
                ];
                for (text, color) in parts {
                    job.append(text, 0.0, egui::TextFormat::simple(font.clone(), color));
                }
            }
        };

        match self.search.show(ui, &labels, &Xml, label)? {
            SearchAsk::Open(object, line) => Some(SideAsk::OpenLocation(object.into(), line)),
            SearchAsk::Replace(only) => {
                replace(work, sel.active, &self.search, only);
                Some(SideAsk::Reload) // the open object may be one of them
            }
        }
    }
}

/// Replaces what `search` found: in the line of hit `only`, or everywhere. See the module docs.
fn replace(work: &mut Work, active: usize, search: &SearchView, only: Option<usize>) {
    let replaced: Vec<(String, String)> = {
        let behavior = &work.workspace.behaviors[active];
        let ours = behavior.texts().filter(|object| !behavior.is_ref(object.id));
        let edits = search.replace_edits(ours.map(|object| (object.id, object.text)), only);
        // In the order of the objects: each one's edits together.
        let mut documents: Vec<&str> = edits.iter().map(|edit| edit.document.as_ref()).collect();
        documents.dedup();
        let texts = documents.into_iter().filter_map(|document| {
            let of_it = edits.iter().filter(|edit| edit.document == document);
            Some((document.to_owned(), DocumentEdit::apply(behavior.text(document)?, of_it)))
        });
        texts.collect()
    };
    for (id, text) in replaced {
        work.workspace.behaviors[active].set_text(&id, &text);
        work.object_changed(&id);
        work.note_pandora_edit(active, &id);
    }
}
