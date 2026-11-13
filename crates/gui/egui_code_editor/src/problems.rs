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
//!
//! The keyboard, as in a Vim file tree: `Ctrl+Shift+L` (the host's key binding `problems.toggle`) shows the
//! panel with it, or hides the panel that has it; there `j` / `k` / `↓` / `↑` / `g` / `G` move the cursor,
//! `Enter` goes to the problem and `Esc` back to the editor.

use std::borrow::Cow;

use crate::{Diagnostic, Severity};

/// The problems of one file or object of the panel.
#[derive(Debug, Clone)]
pub struct ProblemGroup<'a> {
    /// e.g. `#0106  0_master`. Empty: the problems are shown without a header (one file).
    pub title: Cow<'a, str>,
    pub problems: &'a [Diagnostic],
}

/// A key of the panel while it has the keyboard. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemsKey {
    /// `j`, Down
    Next,
    /// `k`, Up
    Previous,
    /// `g`, Home
    First,
    /// `G`, End
    Last,
    /// `Enter`: to the problem under the cursor.
    Go,
    /// `Esc`: back to the editor.
    Leave,
}

/// Where the keyboard is in the panel: kept by the host between frames.
///
/// e.g. `ProblemsCursor { focused: true, row: 2 }`: the third problem of all groups shown, counted
/// across them, has the cursor.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProblemsCursor {
    /// The panel has the keyboard, not the editor.
    pub focused: bool,
    /// Of all the problems shown, in their order.
    pub row: usize,
}

impl ProblemsCursor {
    /// Applies `key` in a panel of `len` problems. Returns the row to go to (`Go`): the keyboard then
    /// leaves the panel, as it does on `Leave`.
    ///
    /// e.g. `Next` on the last row stays there; `Go` on row 2 -> `Some(2)`.
    pub const fn apply(&mut self, key: ProblemsKey, len: usize) -> Option<usize> {
        let last = len.saturating_sub(1);
        match key {
            ProblemsKey::Next => self.row = if self.row < last { self.row + 1 } else { last },
            ProblemsKey::Previous => self.row = self.row.saturating_sub(1),
            ProblemsKey::First => self.row = 0,
            ProblemsKey::Last => self.row = last,
            ProblemsKey::Leave => self.focused = false,
            ProblemsKey::Go => {
                self.focused = false;
                return if len == 0 { None } else { Some(if self.row < last { self.row } else { last }) };
            }
        }
        None
    }
}

/// The keys of the panel pressed this frame, consumed so the editor does not get them.
fn read_keys(ctx: &egui::Context) -> Vec<ProblemsKey> {
    use egui::{Key, Modifiers};
    // `G` before `g`: `consume_key` without modifiers also takes the Shift variant.
    const BINDINGS: [(Modifiers, Key, ProblemsKey); 10] = [
        (Modifiers::SHIFT, Key::G, ProblemsKey::Last),
        (Modifiers::NONE, Key::J, ProblemsKey::Next),
        (Modifiers::NONE, Key::ArrowDown, ProblemsKey::Next),
        (Modifiers::NONE, Key::K, ProblemsKey::Previous),
        (Modifiers::NONE, Key::ArrowUp, ProblemsKey::Previous),
        (Modifiers::NONE, Key::G, ProblemsKey::First),
        (Modifiers::NONE, Key::Home, ProblemsKey::First),
        (Modifiers::NONE, Key::End, ProblemsKey::Last),
        (Modifiers::NONE, Key::Enter, ProblemsKey::Go),
        (Modifiers::NONE, Key::Escape, ProblemsKey::Leave),
    ];
    ctx.input_mut(|input| {
        let mut keys = Vec::new();
        for (modifiers, key, action) in BINDINGS {
            while input.consume_key(modifiers, key) {
                keys.push(action);
            }
        }
        keys
    })
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
    cursor: Option<&'a mut ProblemsCursor>,
}

impl<'a> ProblemsPanel<'a> {
    /// `id_salt`: of the panel, one per host. `title`: e.g. `Problems`.
    #[inline]
    pub const fn new(id_salt: &'a str, title: &'a str) -> Self {
        Self { id_salt, title, empty: "", default_height: 140.0, cursor: None }
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

    /// The keyboard's place in it. Without one, only the mouse works it.
    #[inline]
    pub const fn cursor(mut self, cursor: &'a mut ProblemsCursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Draws it at the bottom of `ui`. `extra` draws more at the end of a row, e.g. a quick fix's
    /// button: it is given `(group, problem)` indices. Returns those of the problem clicked.
    pub fn show(
        mut self,
        ui: &mut egui::Ui,
        groups: &[ProblemGroup<'_>],
        mut extra: impl FnMut(&mut egui::Ui, usize, usize),
    ) -> Option<(usize, usize)> {
        let mut clicked = None;
        let len = groups.iter().map(|group| group.problems.len()).sum();
        // The keys come before the rows: a moved cursor is drawn, and scrolled to, this frame.
        let (mut moved, mut went) = (false, None);
        if let Some(cursor) = self.cursor.as_deref_mut().filter(|cursor| cursor.focused) {
            for key in read_keys(ui.ctx()) {
                went = cursor.apply(key, len);
                moved = true;
                if !cursor.focused {
                    break; // `Go` / `Leave`: the keys after them are the editor's.
                }
            }
        }
        let (cursor_row, focused) = self.cursor.as_deref().map_or((usize::MAX, false), |c| (c.row, c.focused));
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
                let mut flat = 0;
                let mut rows = |ui: &mut egui::Ui, at: usize, group: &ProblemGroup<'_>| {
                    for (index, problem) in group.problems.iter().enumerate() {
                        let under_cursor = focused && flat == cursor_row;
                        let row = problem_row(ui, problem, under_cursor, |ui| extra(ui, at, index));
                        if under_cursor && moved {
                            row.response.scroll_to_me(None);
                        }
                        if row.inner || went == Some(flat) {
                            clicked = Some((at, index, flat));
                        }
                        flat += 1;
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
        let (at, index, flat) = clicked?;
        if let Some(cursor) = self.cursor {
            (cursor.row, cursor.focused) = (flat, false);
        }
        Some((at, index))
    }
}

/// `✖ Unknown field.  Ln 3, Col 4  [extra]`, framed if `under_cursor`. `inner`: it was clicked.
fn problem_row(
    ui: &mut egui::Ui,
    problem: &Diagnostic,
    under_cursor: bool,
    extra: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<bool> {
    let row = ui.horizontal_wrapped(|ui| {
        ui.colored_label(problem.severity.color(ui.visuals()), problem.severity.icon());
        let clicked = ui.link(&problem.message).clicked();
        ui.weak(format!("Ln {}, Col {}", problem.line + 1, problem.columns.start + 1));
        extra(ui);
        clicked
    });
    if under_cursor {
        let stroke = ui.visuals().selection.stroke;
        ui.painter().rect_stroke(row.response.rect, 2.0, stroke, egui::StrokeKind::Inside);
    }
    row
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

    /// The cursor stays in the list; `Go` and `Leave` give the keyboard back to the editor.
    #[test]
    fn the_cursor_moves_in_the_list() {
        let mut cursor = ProblemsCursor { focused: true, row: 0 };
        assert_eq!(cursor.apply(ProblemsKey::Previous, 3), None);
        assert_eq!(cursor.row, 0);
        cursor.apply(ProblemsKey::Last, 3);
        cursor.apply(ProblemsKey::Next, 3);
        assert_eq!(cursor.row, 2, "stops at the end");
        cursor.apply(ProblemsKey::First, 3);
        cursor.apply(ProblemsKey::Next, 3);
        assert_eq!((cursor.apply(ProblemsKey::Go, 3), cursor.focused), (Some(1), false));

        let mut cursor = ProblemsCursor { focused: true, row: 5 };
        assert_eq!(cursor.apply(ProblemsKey::Go, 2), Some(1), "the list got shorter");
        let mut cursor = ProblemsCursor { focused: true, row: 0 };
        assert_eq!(cursor.apply(ProblemsKey::Go, 0), None);
        cursor.focused = true;
        cursor.apply(ProblemsKey::Leave, 3);
        assert!(!cursor.focused);
    }
}
