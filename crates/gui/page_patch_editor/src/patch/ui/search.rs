//! The side bar's search: a field and the matching lines. The logic is in [`crate::patch::search`].
//!
//! ```text
//! [$eventID[moveStart]$     ]  3
//! #0106:12  <hkparam name="id">$eventID[moveStart]$</hkparam>     <- a click opens that line
//! ```

use egui_widgets::shadcn::TextEdit;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::PatchTool;

impl PatchTool {
    pub(in crate::patch) fn search_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        let field = TextEdit::singleline(&mut self.search.query)
            .hint_text(i18n.t(I18nKey::NemesisSearchHint))
            .desired_width(ui.available_width());
        let field = ui.add(field);
        if std::mem::take(&mut self.focus_search) {
            field.request_focus();
        }

        let generation = self.names().graph.generation();
        self.search.refresh(self.active, generation, self.workspace.behaviors[self.active].texts());
        let count = self.search.hits.len();
        ui.weak(if self.search.is_cut() { format!("{count}+") } else { count.to_string() });

        let mut picked = None;
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
        egui::ScrollArea::both().id_salt("nemesis_search").auto_shrink(false).show_rows(
            ui,
            row_height,
            count,
            |ui, range| {
                for at in range {
                    let hit = &self.search.hits[at];
                    let label =
                        egui::RichText::new(format!("{}:{}  {}", hit.object, hit.line + 1, hit.text)).monospace();
                    if ui.add(egui::Button::selectable(false, label)).clicked() {
                        picked = Some(at);
                    }
                }
            },
        );
        if let Some(at) = picked {
            let (object, line) = (self.search.hits[at].object.clone(), self.search.hits[at].line);
            let current = self.selected.clone().unwrap_or_default();
            self.open_location(&current, &object, line);
        }
    }
}
