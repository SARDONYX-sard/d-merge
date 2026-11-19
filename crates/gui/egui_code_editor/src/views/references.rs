//! The references found (`Shift+F12`, Vim `gD`), as VS Code's "peek references": a panel under the
//! editor, the place in a preview on the left and the list on the right, by document.
//!
//! ```text
//! ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔  <- the accent line
//!  #0106 — 3 references                                          ✖
//! ┌──────────────────────────────────────┬─────────────────────────┐
//! │ 11   <hkparam name="generator">     │ ▾ #0105  hkbClipGen…  2 │
//! │ 12   <hkparam name="id">#0106</…    │     12  <hkparam name…  │ <- selected: previewed
//! │ 13   ...                             │     30  ...             │
//! │ (read only, the line at the cursor)  │ ▾ #0240  hkbStateMa…  1 │
//! └──────────────────────────────────────┴─────────────────────────┘
//! ```
//!
//! | key                              | does                                          |
//! | -------------------------------- | --------------------------------------------- |
//! | `↑` `↓` / `k` `j`                | another reference, shown in the preview       |
//! | `Enter`, a double click          | goes there: [`PeekAsk::Open`], and it closes  |
//! | `Esc`, `✖`                       | closes: [`PeekAsk::Close`]                    |
//! | `F4` / `Shift+F4` (the host's)   | [`ReferenceList::step`]: the next / previous  |
//!
//! The list has the keyboard while it is open, so that the editor's keys (Vim's) do not take them.
//!
//! ```text
//! Some(Navigation::References(found)) => list = Some(found.into()),   // from the editor's output
//! match ReferenceList::peek(ui, &mut list, &input) { Some(PeekAsk::Open(document, line)) => .. }
//! ```

use crate::{CodeEditor, CodeEditorState, Language, References};

/// The share of the panel's width the preview takes; the list has the rest.
const PREVIEW_SHARE: f32 = 0.62;
/// The width of the accent line at the top of the panel.
const ACCENT_LINE: f32 = 2.0;

/// One reference.
#[derive(Debug)]
struct Row {
    document: Box<str>,
    /// 0-based.
    line: usize,
    /// e.g., `hkbClipGenerator`
    detail: Box<str>,
    /// The line's text, trimmed: read with the documents' texts the first time the panel is drawn.
    snippet: Option<Box<str>>,
}

/// What the preview shows: the document of the reference selected, read only.
#[derive(Debug, Default)]
struct Preview {
    /// e.g., `#0105`
    document: Box<str>,
    text: String,
    state: CodeEditorState,
    /// The line to put the cursor at on the next draw.
    line: Option<usize>,
}

/// See the module docs. Kept across frames, so it owns what the language lent.
#[derive(Debug)]
pub struct ReferenceList {
    /// e.g., `#0106`
    symbol: Box<str>,
    /// In the order the language found them: by document, then by line.
    rows: Vec<Row>,
    /// The row selected: shown in the preview, and gone to by [`Self::step`]. `None`: no rows.
    current: Option<usize>,
    /// The list scrolls to `current` once, and the preview goes to it.
    moved: bool,
    preview: Preview,
    /// The list takes the keyboard on its next draw: it was just opened.
    take_keyboard: bool,
}

/// What the panel asked of the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeekAsk {
    /// Go to `(document, line)`: the panel has closed.
    Open(String, usize),
    /// `Esc` or `✖`: the panel has closed, the keyboard goes back to the editor.
    Close,
}

/// What the panel draws with: the documents' texts (e.g., an object's, by its id), and their
/// language, for the preview.
pub struct PeekInput<'a> {
    pub texts: &'a dyn Fn(&str) -> Option<String>,
    pub language: &'a dyn Language,
}

impl std::fmt::Debug for PeekInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeekInput").finish_non_exhaustive()
    }
}

impl From<References<'_>> for ReferenceList {
    fn from(references: References<'_>) -> Self {
        let rows: Vec<Row> = references
            .locations
            .into_iter()
            .map(|location| Row {
                document: location.document.into(),
                line: location.line,
                detail: location.detail.into(),
                snippet: None,
            })
            .collect();
        let current = (!rows.is_empty()).then_some(0);
        Self {
            symbol: references.symbol.into(),
            rows,
            current,
            moved: true,
            preview: Preview::default(),
            take_keyboard: true,
        }
    }
}

impl ReferenceList {
    /// The next row (`forward`) or the previous one, around the ends, as the one selected now.
    /// Returns its `(document, line)`, to go there.
    ///
    /// e.g. 3 rows, at row 2: forward -> row 0
    pub fn step(&mut self, forward: bool) -> Option<(&str, usize)> {
        let count = self.rows.len();
        if count == 0 {
            return None;
        }
        let next = match (self.current, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(at), true) => (at + 1) % count,
            (Some(at), false) => (at + count - 1) % count,
        };
        self.select(next);
        self.current_place()
    }

    /// Row `index` is the one selected now.
    const fn select(&mut self, index: usize) {
        self.current = Some(index);
        self.moved = true;
    }

    /// The `(document, line)` of the row selected.
    fn current_place(&self) -> Option<(&str, usize)> {
        let row = self.rows.get(self.current?)?;
        Some((&row.document, row.line))
    }

    /// Draws the panel of `list` at the bottom of `ui`: call it before what fills the rest. Returns
    /// what was asked; `list` is `None` once it has closed.
    pub fn peek(ui: &mut egui::Ui, list: &mut Option<Self>, input: &PeekInput<'_>) -> Option<PeekAsk> {
        let references = list.as_mut()?;
        references.read_snippets(input.texts);
        references.load_preview(input.texts);

        let panel = egui::Panel::bottom("references_peek").resizable(true).default_size(260.0);
        let asked = panel.show(ui, |ui| references.panel_ui(ui, input.language)).inner;
        if asked.is_some() {
            *list = None;
        }
        asked
    }

    /// The text of each row's line, once.
    fn read_snippets(&mut self, texts: &dyn Fn(&str) -> Option<String>) {
        if self.rows.iter().all(|row| row.snippet.is_some()) {
            return;
        }
        // The rows are by document: each document is read once.
        let mut read: Option<(&str, Option<String>)> = None;
        for row in &mut self.rows {
            if read.as_ref().is_none_or(|(document, _)| *document != &*row.document) {
                read = Some((&row.document, texts(&row.document)));
            }
            let text = read.as_ref().and_then(|(_, text)| text.as_deref());
            let line = text.and_then(|text| text.lines().nth(row.line)).unwrap_or_default();
            row.snippet = Some(line.trim().into());
        }
    }

    /// When the selection moved: the preview shows its row's document, the cursor at its line.
    fn load_preview(&mut self, texts: &dyn Fn(&str) -> Option<String>) {
        if !self.moved {
            return;
        }
        let Some((document, line)) = self.current_place() else { return };
        if *self.preview.document != *document {
            let text = texts(document).unwrap_or_default();
            self.preview = Preview { document: document.into(), text, ..Preview::default() };
        }
        self.preview.line = Some(line);
    }

    /// The accent line and the title, then the preview and the list side by side.
    fn panel_ui(&mut self, ui: &mut egui::Ui, language: &dyn Language) -> Option<PeekAsk> {
        let accent = egui::Stroke::new(ACCENT_LINE, ui.visuals().selection.stroke.color);
        ui.painter().hline(ui.max_rect().x_range(), ui.max_rect().top(), accent);

        let closed = self.title_ui(ui);
        let height = ui.available_height();
        let preview_width = ui.available_width() * PREVIEW_SHARE;
        let mut asked = None;
        ui.horizontal_top(|ui| {
            ui.allocate_ui(egui::vec2(preview_width, height), |ui| self.preview_ui(ui, language));
            ui.separator();
            ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui| asked = self.list_ui(ui));
        });
        self.moved = false;
        if closed { Some(PeekAsk::Close) } else { asked }
    }

    /// `#0106 — 3 references ............ ✖`. Returns whether `✖` was clicked.
    fn title_ui(&self, ui: &mut egui::Ui) -> bool {
        let mut closed = false;
        ui.horizontal(|ui| {
            ui.strong(&*self.symbol);
            ui.weak(format!("— {} references", self.rows.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                closed = ui.add(egui::Button::new("✖").frame(false)).clicked();
            });
        });
        closed
    }

    /// The document of the selected row, read only, its cursor at the row's line.
    fn preview_ui(&mut self, ui: &mut egui::Ui, language: &dyn Language) {
        let id_salt = ("references_peek_preview", &*self.preview.document);
        if let Some(line) = self.preview.line.take() {
            let at = crate::text_pos::LinePos { line, column: 0 }.to_char_index(&self.preview.text);
            crate::place_cursor(ui.ctx(), crate::editor_id(id_salt), at);
        }
        let scroll = egui::ScrollArea::both().id_salt("references_peek_preview_scroll").auto_shrink(false);
        scroll.show(ui, |ui| {
            let editor = CodeEditor::new(id_salt, &mut self.preview.text).language(language).read_only(true);
            editor.show(ui, &mut self.preview.state);
        });
    }

    /// The references by document, the selected one marked; and the keys while it has the keyboard.
    fn list_ui(&mut self, ui: &mut egui::Ui) -> Option<PeekAsk> {
        let area = ui.interact(
            ui.max_rect(),
            ui.scope_id().with("references_peek_list"),
            egui::Sense::focusable_noninteractive(),
        );
        if std::mem::take(&mut self.take_keyboard) {
            area.request_focus();
        }
        let mut asked = if area.has_focus() { self.keys(ui) } else { None };

        let mut clicked = None;
        egui::ScrollArea::vertical().id_salt("references_peek_list_scroll").auto_shrink(false).show(ui, |ui| {
            let mut document: Option<&str> = None;
            for (index, row) in self.rows.iter().enumerate() {
                if document != Some(&*row.document) {
                    document = Some(&row.document);
                    let count = self.rows.iter().filter(|other| other.document == row.document).count();
                    document_header(ui, row, count);
                }

                let is_current = self.current == Some(index);
                let response = reference_row(ui, row, is_current);
                if is_current && self.moved {
                    response.scroll_to_me(None);
                }
                if response.clicked() {
                    clicked = Some(index);
                }
                if response.double_clicked() {
                    asked = Some(PeekAsk::Open(row.document.to_string(), row.line));
                }
            }
        });
        if let Some(index) = clicked {
            self.select(index);
            area.request_focus();
        }
        asked
    }

    /// `↑` `↓` `j` `k` select, `Enter` opens, `Esc` closes. Returns what was asked.
    fn keys(&mut self, ui: &egui::Ui) -> Option<PeekAsk> {
        use egui::{Key, Modifiers};
        let pressed = |key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));

        if pressed(Key::ArrowDown) || pressed(Key::J) {
            self.step(true);
        }
        if pressed(Key::ArrowUp) || pressed(Key::K) {
            self.step(false);
        }
        if pressed(Key::Escape) {
            return Some(PeekAsk::Close);
        }
        if pressed(Key::Enter) {
            let (document, line) = self.current_place()?;
            return Some(PeekAsk::Open(document.to_owned(), line));
        }
        None
    }
}

/// `▾ #0105  hkbClipGenerator  2`
fn document_header(ui: &mut egui::Ui, row: &Row, count: usize) {
    ui.horizontal(|ui| {
        ui.weak("▾");
        ui.strong(&*row.document);
        ui.weak(&*row.detail);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.weak(count.to_string());
        });
    });
}

/// `    12  <hkparam name="id">#0106</hkparam>`, selected or not. People count lines from 1.
fn reference_row(ui: &mut egui::Ui, row: &Row, is_current: bool) -> egui::Response {
    let snippet = row.snippet.as_deref().unwrap_or_default();
    let text = egui::RichText::new(format!("    {:>4}  {snippet}", row.line + 1)).monospace();
    let button = egui::Button::selectable(is_current, text).truncate();
    ui.add_sized([ui.available_width(), 0.0], button)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;

    fn list(lines: &[usize]) -> ReferenceList {
        let at = |&line| Location { document: "#a".into(), line, detail: "".into() };
        ReferenceList::from(References { symbol: "x".into(), locations: lines.iter().map(at).collect() })
    }

    /// As Vim's `]q` / `[q`: from row to row, around the ends. It opens at the first.
    #[test]
    fn a_step_goes_to_the_next_or_the_previous_row() {
        let mut refs = list(&[1, 2, 3]);
        assert_eq!(refs.current_place(), Some(("#a", 1)), "the first, selected at once");
        assert_eq!(refs.step(false), Some(("#a", 3)), "around the start");
        assert_eq!(refs.step(true), Some(("#a", 1)), "around the end");
        assert_eq!(refs.step(true), Some(("#a", 2)));

        assert_eq!(list(&[]).step(true), None);
    }

    /// The rows show their lines' texts, read once; the preview, the selected row's document.
    #[test]
    fn a_row_shows_its_line() {
        let mut refs = list(&[1]);
        refs.read_snippets(&|_| Some("a\n\t\tb\nc".to_owned()));
        assert_eq!(refs.rows[0].snippet.as_deref(), Some("b"));

        refs.load_preview(&|_| Some("text".to_owned()));
        assert_eq!((&*refs.preview.document, refs.preview.line), ("#a", Some(1)));
    }
}
