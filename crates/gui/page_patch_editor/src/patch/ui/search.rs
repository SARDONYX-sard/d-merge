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

use crate::patch::PatchTool;

impl PatchTool {
    pub(in crate::patch) fn search_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        if std::mem::take(&mut self.focus_search) {
            self.search.focus();
        }
        let generation = self.names().graph.generation();
        let behavior = &self.workspace.behaviors[self.active];
        self.search.refresh((self.active, generation), behavior.texts().map(|object| (object.id, object.text)));
        let labels = SearchLabels {
            hint: i18n.t(I18nKey::NemesisSearchHint),
            regex_hover: i18n.t(I18nKey::SearchRegexHover),
            results_in: i18n.t(I18nKey::SearchResultsIn),
            documents: i18n.t(I18nKey::SearchObjects),
        };
        // `#0106  hkbModifierList  GetUpModList`
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let label = |job: &mut LayoutJob, id: &str, visuals: &egui::Visuals| {
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
        };
        let open = self.search.show(ui, &labels, &Xml, label);
        if let Some((object, line)) = open {
            let current = self.selected.clone().unwrap_or_default();
            self.open_location(&current, &object, line);
        }
    }
}
