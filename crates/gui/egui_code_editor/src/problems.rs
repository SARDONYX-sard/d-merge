//! The problems of what is edited, as VS Code shows them: their counts in the status bar, and the
//! Problems panel the counts show and hide. For any host of the editor (the patch editor, hkanno).
//!
//! ```text
//! status bar:  ✖ 2  ⚠ 5 | ...           <- `status_counts`: a click shows / hides the panel
//!
//! Problems  ✖ 2  ⚠ 5                     <- `ProblemsPanel::show`
//! ▼ #0106  0_master  (2)                  <- a group (a file, an object); no header if untitled
//!     ✖ Unknown field `typoField`.  Ln 3, Col 4   [fix]   <- a click goes there; `extra` adds [fix]
//!     ⚠ `#0002` does not exist.     Ln 9, Col 5
//! ```

use std::borrow::Cow;

use crate::{Diagnostic, Severity};

/// The problems of one file or object of the panel.
#[derive(Debug, Clone)]
pub struct ProblemGroup<'a> {
    /// e.g. `#0106  0_master`. Empty: the problems are shown without a header (one file).
    pub title: Cow<'a, str>,
    pub problems: &'a [Diagnostic],
}

/// `(errors, warnings)` of `problems`.
#[must_use]
pub fn counts<'a>(problems: impl IntoIterator<Item = &'a Diagnostic>) -> (usize, usize) {
    problems.into_iter().fold((0, 0), |(errors, warnings), problem| match problem.severity {
        Severity::Error => (errors + 1, warnings),
        Severity::Warning => (errors, warnings + 1),
    })
}

/// `✖ 2  ⚠ 5` for the status bar, gray when there is nothing, red / yellow else. Its response is
/// clicked to show or hide the panel.
pub fn status_counts(ui: &mut egui::Ui, (errors, warnings): (usize, usize)) -> egui::Response {
    let visuals = ui.visuals();
    let quiet = errors + warnings == 0;
    let color = |severity: Severity| if quiet { visuals.weak_text_color() } else { severity.color(visuals) };
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    let error = format!("{} {errors}", Severity::Error.icon());
    let warning = format!("  {} {warnings}", Severity::Warning.icon());
    job.append(&error, 0.0, egui::TextFormat::simple(font.clone(), color(Severity::Error)));
    job.append(&warning, 0.0, egui::TextFormat::simple(font, color(Severity::Warning)));
    ui.add(egui::Button::new(job).frame(false))
}

/// The Problems panel. See the module docs.
#[derive(Debug)]
#[must_use = "Call `show` to draw it"]
pub struct ProblemsPanel<'a> {
    id_salt: &'a str,
    title: &'a str,
    empty: &'a str,
    default_height: f32,
}

impl<'a> ProblemsPanel<'a> {
    /// `id_salt`: of the panel, one per host. `title`: e.g. `Problems`.
    #[inline]
    pub const fn new(id_salt: &'a str, title: &'a str) -> Self {
        Self { id_salt, title, empty: "", default_height: 140.0 }
    }

    /// What it says without any problem, e.g. `No problems have been found.`
    #[inline]
    pub const fn empty_text(mut self, empty: &'a str) -> Self {
        self.empty = empty;
        self
    }

    /// Its height until it is resized.
    #[inline]
    pub const fn default_height(mut self, height: f32) -> Self {
        self.default_height = height;
        self
    }

    /// Draws it at the bottom of `ui`. `extra` draws more at the end of a row, e.g. a quick fix's
    /// button: it is given `(group, problem)` indices. Returns those of the problem clicked.
    pub fn show(
        self,
        ui: &mut egui::Ui,
        groups: &[ProblemGroup<'_>],
        mut extra: impl FnMut(&mut egui::Ui, usize, usize),
    ) -> Option<(usize, usize)> {
        let mut clicked = None;
        egui::Panel::bottom(self.id_salt).resizable(true).default_size(self.default_height).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(self.title);
                let (errors, warnings) = counts(groups.iter().flat_map(|group| group.problems));
                ui.colored_label(Severity::Error.color(ui.visuals()), format!("{} {errors}", Severity::Error.icon()));
                ui.colored_label(
                    Severity::Warning.color(ui.visuals()),
                    format!("{} {warnings}", Severity::Warning.icon()),
                );
            });
            if groups.iter().all(|group| group.problems.is_empty()) {
                ui.weak(self.empty);
                return;
            }
            egui::ScrollArea::vertical().id_salt((self.id_salt, "scroll")).auto_shrink(false).show(ui, |ui| {
                let mut rows = |ui: &mut egui::Ui, at: usize, group: &ProblemGroup<'_>| {
                    for (index, problem) in group.problems.iter().enumerate() {
                        if problem_row(ui, problem, |ui| extra(ui, at, index)) {
                            clicked = Some((at, index));
                        }
                    }
                };
                for (at, group) in groups.iter().enumerate().filter(|(_, group)| !group.problems.is_empty()) {
                    if group.title.is_empty() {
                        rows(ui, at, group);
                        continue;
                    }
                    let title = format!("{}  ({})", group.title, group.problems.len());
                    egui::CollapsingHeader::new(egui::RichText::new(title).monospace())
                        .id_salt((self.id_salt, at, &group.title))
                        .default_open(true)
                        .show(ui, |ui| rows(ui, at, group));
                }
            });
        });
        clicked
    }
}

/// `✖ Unknown field.  Ln 3, Col 4  [extra]`. Returns whether it was clicked.
fn problem_row(ui: &mut egui::Ui, problem: &Diagnostic, extra: impl FnOnce(&mut egui::Ui)) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(problem.severity.color(ui.visuals()), problem.severity.icon());
        let clicked = ui.link(&problem.message).clicked();
        ui.weak(format!("Ln {}, Col {}", problem.line + 1, problem.columns.start + 1));
        extra(ui);
        clicked
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(severity: Severity) -> Diagnostic {
        Diagnostic { line: 0, columns: 0..1, severity, message: String::new() }
    }

    #[test]
    fn counts_errors_and_the_rest() {
        let problems = [problem(Severity::Error), problem(Severity::Warning), problem(Severity::Error)];
        assert_eq!(counts(&problems), (2, 1));
        assert_eq!(counts(&[]), (0, 0));
    }
}
