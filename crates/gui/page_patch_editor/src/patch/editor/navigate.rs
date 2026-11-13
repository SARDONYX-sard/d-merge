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
//! | `Alt+Left` / `Alt+Right`  | The same, also the `<-` `->` buttons of the tab bar  |

use egui_code_editor::References;

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
pub(in crate::patch) struct ReferenceList {
    title: Box<str>,
    rows: Vec<ReferenceRow>,
}

impl ReferenceList {
    /// e.g., `References: #0106 (3)`
    pub(in crate::patch) fn title(&self) -> &str {
        &self.title
    }

    pub(in crate::patch) const fn len(&self) -> usize {
        self.rows.len()
    }

    pub(in crate::patch) fn label(&self, index: usize) -> &str {
        &self.rows[index].label
    }

    /// `(document, line)` of row `index`.
    pub(in crate::patch) fn location(&self, index: usize) -> Option<(&str, usize)> {
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
    fn reference_rows_are_one_based_for_people() {
        let location = Location { document: "#0106".into(), line: 11, detail: "hkbClipGenerator".into() };
        let list = ReferenceList::from(References { symbol: "#0105".into(), locations: vec![location] });
        assert_eq!(list.title(), "References: #0105 (1)");
        assert_eq!(list.label(0), "#0106:12  hkbClipGenerator");
        assert_eq!(list.location(0), Some(("#0106", 11)));
    }
}
