//! Moving between objects: the visited-objects history and the references list.
//!
//! The editor resolves `F12`, `Shift+F12`, `Ctrl+click`, Vim `gd` / `gD` and its right-click menu
//! through [`egui_code_editor::Language`] (see `language`). This module keeps the result:
//!
//! | Key                       | Action                                               |
//! | ------------------------- | ---------------------------------------------------- |
//! | `F12`, `Ctrl+click`, `gd` | Open the object / line that defines the symbol       |
//! | `Shift+F12`, `gD`         | List the places that use it (click one to go)        |
//! | Vim `Ctrl+O` / `Ctrl+I`   | Back / forward through the visited objects           |

use egui_code_editor::References;

/// Visited objects, for `Ctrl+O` / `Ctrl+I`.
#[derive(Debug, Default)]
pub(super) struct History {
    back: Vec<String>,
    forward: Vec<String>,
}

impl History {
    /// Leaving `from` for another object.
    pub(super) fn visit(&mut self, from: &str) {
        if self.back.last().map(String::as_str) != Some(from) {
            self.back.push(from.to_owned());
        }
        self.forward.clear();
    }

    /// The object to go back (`older`) / forward to, from `current`.
    pub(super) fn walk(&mut self, older: bool, current: &str) -> Option<String> {
        let (from, to) = if older { (&mut self.back, &mut self.forward) } else { (&mut self.forward, &mut self.back) };
        let target = from.pop()?;
        to.push(current.to_owned());
        Some(target)
    }
}

/// One line of the references window.
#[derive(Debug)]
struct ReferenceRow {
    document: Box<str>,
    line: usize,
    /// e.g., `#0106:12  hkbClipGenerator`
    label: Box<str>,
}

/// The open references window. Kept across frames, so it owns what the language lent.
#[derive(Debug)]
pub(super) struct ReferenceList {
    title: Box<str>,
    rows: Vec<ReferenceRow>,
}

impl ReferenceList {
    /// e.g., `References: #0106 (3)`
    pub(super) fn title(&self) -> &str {
        &self.title
    }

    pub(super) const fn len(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn label(&self, index: usize) -> &str {
        &self.rows[index].label
    }

    /// `(document, line)` of row `index`.
    pub(super) fn location(&self, index: usize) -> Option<(&str, usize)> {
        self.rows.get(index).map(|row| (&*row.document, row.line))
    }
}

impl From<References<'_>> for ReferenceList {
    fn from(references: References<'_>) -> Self {
        let rows: Vec<ReferenceRow> = references
            .locations
            .into_iter()
            .map(|location| {
                let label = format!("{}:{}  {}", location.document, location.line + 1, location.detail);
                ReferenceRow { document: location.document.into(), line: location.line, label: label.into() }
            })
            .collect();
        Self { title: format!("References: {} ({})", references.symbol, rows.len()).into(), rows }
    }
}

#[cfg(test)]
mod tests {
    use egui_code_editor::Location;

    use super::*;

    #[test]
    fn history_goes_back_and_forward() {
        let mut history = History::default();
        history.visit("#a"); // a -> b
        history.visit("#b"); // b -> c
        assert_eq!(history.walk(true, "#c").as_deref(), Some("#b"));
        assert_eq!(history.walk(true, "#b").as_deref(), Some("#a"));
        assert_eq!(history.walk(true, "#a"), None);
        assert_eq!(history.walk(false, "#a").as_deref(), Some("#b"));

        history.visit("#b"); // a new jump drops the forward list
        assert_eq!(history.walk(false, "#z"), None);
    }

    #[test]
    fn reference_rows_are_one_based_for_people() {
        let location = Location { document: "#0106".into(), line: 11, detail: "hkbClipGenerator".into() };
        let list = ReferenceList::from(References { symbol: "#0105".into(), locations: vec![location] });
        assert_eq!(list.title(), "References: #0105 (1)");
        assert_eq!(list.label(0), "#0106:12  hkbClipGenerator");
        assert_eq!(list.location(0), Some(("#0106", 11)));
    }
}
