//! The side bar's search: [`egui_code_editor::SearchView`] over the objects of the active behavior,
//! each found line under its object, named as the tree names it:
//!
//! ```text
//! ▼ #0106  hkbModifierList  GetUpModList      2
//!     <hkparam name="id">$eventID[moveStart]$</hkparam>
//! ```

use egui::text::LayoutJob;
use egui_code_editor::{SearchLabels, languages::Xml};
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
            hint: i18n.t(I18nKey::NemesisSearchHint),
            regex_hover: i18n.t(I18nKey::SearchRegexHover),
            results_in: i18n.t(I18nKey::SearchResultsIn),
            documents: i18n.t(I18nKey::SearchObjects),
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

        let (object, line) = self.search.show(ui, &labels, &Xml, label)?;
        Some(SideAsk::OpenLocation(object.to_string(), line))
    }
}
