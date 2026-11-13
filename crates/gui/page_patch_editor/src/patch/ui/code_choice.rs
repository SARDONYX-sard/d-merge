//! Which mod code to open, when a mod has several of them.
//!
//! A work is of one mod code, and a mod manager's mod may hold many:
//! ```text
//! mods/Elden Counter/Nemesis_Engine/mod/ecdmg
//! mods/Elden Counter/Nemesis_Engine/mod/ecnpc
//! ```
//! Opening one of them without asking would hide the others.

use std::path::PathBuf;

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{PatchTool, workspace::ModCode};

/// The list stops growing at this height, and scrolls.
const MAX_LIST_HEIGHT: f32 = 320.0;

/// The question that is open. e.g. `root`: `D:/mods/Elden Counter`, `codes`: `ecdmg`, `ecnpc`
#[derive(Debug)]
pub(in crate::patch) struct ModCodeChoice {
    pub(in crate::patch) root: PathBuf,
    pub(in crate::patch) codes: Vec<ModCode>,
}

/// What a row says: the name of the mod is what one knows it by. e.g. `ecnpc  Elden Counter NPC`
fn row_label(code: &ModCode) -> String {
    if code.name.is_empty() { code.code.clone() } else { format!("{}  {}", code.code, code.name) }
}

impl PatchTool {
    /// Draws the window while [`Self::code_choice`] is set, and opens what is clicked.
    pub(in crate::patch) fn code_choice_window(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18nMap,
        notify: &mut Notification,
    ) {
        let Some(choice) = &self.panels.code_choice else { return };

        let mut open = true;
        let mut picked = None;
        egui_widgets::quick_pick_window(ctx, i18n.t(I18nKey::NemesisChooseModCode)).open(&mut open).show(ctx, |ui| {
            ui.weak(choice.root.display().to_string());
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(MAX_LIST_HEIGHT).show(ui, |ui| {
                for code in &choice.codes {
                    if ui.button(row_label(code)).clicked() {
                        picked = Some(code.code.clone());
                    }
                }
            });
        });

        if let Some(code) = picked {
            self.open_chosen_code(&code, notify, i18n);
        } else if !open {
            self.panels.code_choice = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_has_the_name_if_there_is_one() {
        let row = |name: &str| row_label(&ModCode { code: "ecnpc".to_owned(), name: name.to_owned() });
        assert_eq!(row("Elden Counter NPC"), "ecnpc  Elden Counter NPC");
        assert_eq!(row(""), "ecnpc");
    }
}
