//! The Problems panel under the editor, like VS Code's:
//!
//! ```text
//! Problems  ✖ 1  ⚠ 1                 (red / yellow)
//! ✖ Unknown field names: "typoField".   Ln 3, Col 3
//! ⚠ `#0002` does not exist.             Ln 9, Col 5
//! ```
//! A click goes to the line. Long messages wrap here, and do not push the panels around.
//! In the editor the same problems are underlined, and shown on hover.

use egui_code_editor::{Diagnostic, Severity};
use gui_shared::i18n::{I18nKey, I18nMap};

/// Draws the panel if there is anything to show. Returns the line of the problem clicked.
pub(in crate::patch) fn problems_panel(ui: &mut egui::Ui, i18n: &I18nMap, problems: &[Diagnostic]) -> Option<usize> {
    if problems.is_empty() {
        return None;
    }
    let mut clicked = None;
    egui::Panel::bottom("nemesis_problems").resizable(true).default_size(110.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.weak(i18n.t(I18nKey::HkannoProblemsLabel));
            let errors = problems.iter().filter(|problem| problem.severity == Severity::Error).count();
            let counts = [(Severity::Error, errors), (Severity::Warning, problems.len() - errors)];
            for (severity, count) in counts {
                ui.colored_label(severity.color(ui.visuals()), format!("{} {count}", severity.icon()));
            }
        });
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for problem in problems {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(problem.severity.color(ui.visuals()), problem.severity.icon());
                    let place = format!("Ln {}, Col {}", problem.line + 1, problem.columns.start + 1);
                    if ui.link(&problem.message).on_hover_text(&place).clicked() {
                        clicked = Some(problem.line);
                    }
                    ui.weak(place);
                });
            }
        });
    });
    clicked
}
