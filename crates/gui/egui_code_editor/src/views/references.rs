//! The references window, as VS Code lists them (`Shift+F12`, Vim `gD`): what a
//! [`crate::Language::references`] found, kept open until closed; a click goes to that line.
//!
//! ```text
//! ┌ References: #0106 (3) ─────────── ✖ ┐
//! │ #0105:12  hkbClipGenerator          │
//! │ #0240:3   hkbStateMachine           │
//! └─────────────────────────────────────┘
//! ```
//! ```text
//! Some(Navigation::References(found)) => list = Some(found.into()),   // from the editor's output
//! if let Some((document, line)) = ReferenceList::window(ctx, &mut list) { /* go there */ }
//! ```

use crate::References;

/// One line of the window.
#[derive(Debug)]
struct Row {
    document: Box<str>,
    line: usize,
    /// e.g., `#0106:12  hkbClipGenerator`
    label: Box<str>,
}

/// See the module docs. Kept across frames, so it owns what the language lent.
#[derive(Debug)]
pub struct ReferenceList {
    /// e.g., `References: #0106 (3)`
    title: Box<str>,
    rows: Vec<Row>,
}

impl From<References<'_>> for ReferenceList {
    fn from(references: References<'_>) -> Self {
        let rows: Vec<Row> = references
            .locations
            .into_iter()
            .map(|location| {
                let label = format!("{}:{}  {}", location.document, location.line + 1, location.detail);
                Row { document: location.document.into(), line: location.line, label: label.into() }
            })
            .collect();
        Self { title: format!("References: {} ({})", references.symbol, rows.len()).into(), rows }
    }
}

impl ReferenceList {
    /// Draws the window of `list`, near the pointer; `✖` sets it to `None`. Returns the
    /// `(document, line)` clicked.
    pub fn window<'a>(ctx: &egui::Context, list: &'a mut Option<Self>) -> Option<(&'a str, usize)> {
        let references = list.as_ref()?;

        let (mut open, mut picked) = (true, None);
        let window = egui_widgets::window_near_pointer(ctx, &*references.title).open(&mut open);
        window.default_height(300.0).show(ctx, |ui| {
            let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
            egui::ScrollArea::vertical().show_rows(ui, row_height, references.rows.len(), |ui, range| {
                for index in range {
                    let text = egui::RichText::new(&*references.rows[index].label).monospace();
                    if ui.add(egui::Button::selectable(false, text)).clicked() {
                        picked = Some(index);
                    }
                }
            });
        });

        if !open {
            *list = None;
            return None;
        }
        let row = list.as_ref()?.rows.get(picked?)?;
        Some((&row.document, row.line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;

    #[test]
    fn reference_rows_are_one_based_for_people() {
        let location = Location { document: "#0106".into(), line: 11, detail: "hkbClipGenerator".into() };
        let list = ReferenceList::from(References { symbol: "#0105".into(), locations: vec![location] });
        assert_eq!(&*list.title, "References: #0105 (1)");
        assert_eq!(&*list.rows[0].label, "#0106:12  hkbClipGenerator");
        assert_eq!((&*list.rows[0].document, list.rows[0].line), ("#0106", 11));
    }
}
