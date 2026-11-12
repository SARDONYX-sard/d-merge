//! The Problems panel under the editor, as VS Code's: the problems of the whole work, under their
//! object. The status bar's `✖ 2  ⚠ 5` shows and hides it. Found by [`crate::patch::problems`].
//!
//! ```text
//! Problems  ✖ 1  ⚠ 1                        (red / yellow)
//! ▼ #0106  0_master  (2)
//!     ✖ Unknown field names: "typoField".    Ln 3     <- a click goes to the line
//!     ⚠ `#0002` does not exist.              Ln 9
//! ```
//! In the editor the problems of the open object are underlined, and shown on hover.

use egui_code_editor::Severity;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    problems::{Problem, counts},
    workspace::Behavior,
};

/// Draws the panel. Returns where a click goes: `(behavior, object, line)`.
pub(in crate::patch) fn problems_panel(
    ui: &mut egui::Ui,
    i18n: &I18nMap,
    problems: &[Problem],
    behaviors: &[Behavior],
) -> Option<(usize, String, usize)> {
    let mut clicked = None;
    egui::Panel::bottom("nemesis_problems").resizable(true).default_size(140.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong(i18n.t(I18nKey::HkannoProblemsLabel));
            let (errors, warnings) = counts(problems);
            for (severity, count) in [(Severity::Error, errors), (Severity::Warning, warnings)] {
                ui.colored_label(severity.color(ui.visuals()), format!("{} {count}", severity.icon()));
            }
        });
        if problems.is_empty() {
            ui.weak(i18n.t(I18nKey::ProblemsNone));
            return;
        }
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for group in problems.chunk_by(|a, b| (a.behavior, &a.id) == (b.behavior, &b.id)) {
                let first = &group[0];
                let behavior = behaviors.get(first.behavior).map_or("", |behavior| behavior.name.as_str());
                let title = format!("{}  {behavior}  ({})", first.id, group.len());
                egui::CollapsingHeader::new(egui::RichText::new(title).monospace())
                    .id_salt(("nemesis_problem_object", first.behavior, &first.id))
                    .default_open(true)
                    .show(ui, |ui| {
                        for problem in group {
                            if problem_row(ui, problem) {
                                clicked = Some((problem.behavior, problem.id.clone(), problem.line));
                            }
                        }
                    });
            }
        });
    });
    clicked
}

/// `✖ Unknown field names: "typoField".   Ln 3`. Returns whether it was clicked.
fn problem_row(ui: &mut egui::Ui, problem: &Problem) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(problem.severity.color(ui.visuals()), problem.severity.icon());
        let clicked = ui.link(&problem.message).clicked();
        ui.weak(format!("Ln {}", problem.line + 1));
        clicked
    })
    .inner
}
