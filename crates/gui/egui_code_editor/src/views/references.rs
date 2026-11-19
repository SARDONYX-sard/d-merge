//! The references found (`Shift+F12`, Vim `gD`), as VS Code's "peek references": opened inside
//! the editor, under the line they were asked from, pushing the lines below it down.
//!
//! ```text
//!  12   <hkparam name="id">#0106</hkparam>        <- the line asked from
//! ▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔  <- the accent line
//!  #0105  hkbClipGenerator — References (3)                         ✖
//! ┌──────────────────────────────────────┬─────────────────────────┐
//! │ 11   <hkparam name="generator">     │ ▾ #0105  hkbClipGen…  2 │
//! │ 12   <hkparam name="id">#0106</…    │     12  <hkparam name…  │ <- selected: previewed
//! │ (read only, the line at the cursor)  │     30  ...             │
//! │                                      │ ▸ #0240  hkbStateMa…  1 │ <- folded
//! └──────────────────────────────────────┴─────────────────────────┘
//! ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁
//!  13   ...                                       <- the lines below, pushed down
//! ```
//!
//! The list is by document, each folded but the first, as VS Code's.
//!
//! | key                         | does                                                     |
//! | --------------------------- | -------------------------------------------------------- |
//! | `↓` `↑` / `j` `k`           | the next / previous line of the list                     |
//! | `g` / `Shift+G`             | the first / last line of the list, as Vim's `gg` / `G`   |
//! | `o`, `Space`                | folds / unfolds the document at the cursor               |
//! | `←` `→` / `h` `l`           | folds / unfolds it                                       |
//! | `Enter`, a double click     | a document: folds / unfolds; a reference: goes there     |
//! | in the preview: a double click, `Enter` | goes to its line: [`PeekAsk::Open`]         |
//! | `Esc`, `✖`                  | closes: [`PeekAsk::Close`]                               |
//! | `F4` / `Shift+F4` (host's)  | [`ReferenceList::step`]: the next / previous reference   |
//!
//! The list has the keyboard while it is open, so that the editor's keys (Vim's) do not take them.
//! The editor draws it: see [`crate::CodeEditor::references`].

use std::ops::Range;

use crate::{CodeEditor, CodeEditorState, Language, References};

/// The share of the peek's width the preview takes; the list has the rest.
const PREVIEW_SHARE: f32 = 0.62;
/// The width of the accent lines above and below the peek.
const ACCENT_LINE: f32 = 2.0;
/// The size of the `✖` that closes it: a target easy to hit, as VS Code's.
const CLOSE_SIZE: f32 = 24.0;
/// The room right of the `✖`: off the edge, and off the host's scroll bar there.
const CLOSE_MARGIN: f32 = 20.0;
/// Default of [`ReferenceList::height`].
const HEIGHT: f32 = 300.0;

/// One reference.
#[derive(Debug)]
struct Row {
    document: Box<str>,
    /// 0-based.
    line: usize,
    /// e.g., `hkbClipGenerator`
    detail: Box<str>,
    /// The line's text, trimmed: read with the documents' texts the first time the peek is drawn.
    snippet: Option<Box<str>>,
}

/// The references of one document: a run of [`Row`]s.
#[derive(Debug)]
struct Group {
    rows: Range<usize>,
    /// Unfolded: its rows are listed.
    open: bool,
}

/// A line of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    /// The document of the group, e.g., `▾ #0105  hkbClipGenerator  2`.
    Group(usize),
    /// A reference.
    Row(usize),
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
    groups: Vec<Group>,
    /// The line of the list the keys are on. `None`: no rows.
    cursor: Option<Item>,
    /// The reference selected: previewed, and gone to by [`Self::step`]. `None`: no rows.
    current: Option<usize>,
    /// The cursor moved: the list scrolls to it once.
    moved: bool,
    /// `current` moved: the preview goes to it on the next draw. The keys move it after the preview
    /// is drawn, so it is not the frame's `moved`.
    stale_preview: bool,
    preview: Preview,
    /// The list takes the keyboard on its next draw: it was just opened.
    take_keyboard: bool,
    /// The line of the editor it opens under: the cursor's when it is first drawn.
    anchor: Option<usize>,
    height: f32,
}

/// What the peek asked of the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeekAsk {
    /// Go to `(document, line)`: the peek has closed.
    Open(String, usize),
    /// `Esc` or `✖`: the peek has closed, the keyboard goes back to the editor.
    Close,
}

/// What the peek draws with: the documents' texts (e.g., an object's, by its id), and their
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

        // A group per run of one document; the first is unfolded, as VS Code's.
        let mut groups: Vec<Group> = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            match groups.last_mut() {
                Some(group) if rows[group.rows.start].document == row.document => group.rows.end = index + 1,
                _ => groups.push(Group { rows: index..index + 1, open: groups.is_empty() }),
            }
        }

        let current = (!rows.is_empty()).then_some(0);
        Self {
            symbol: references.symbol.into(),
            rows,
            groups,
            cursor: current.map(Item::Row),
            current,
            moved: true,
            stale_preview: true,
            preview: Preview::default(),
            take_keyboard: true,
            anchor: None,
            height: HEIGHT,
        }
    }
}

impl ReferenceList {
    /// How tall the peek is, in points (default 300).
    #[inline]
    #[must_use]
    pub const fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Every document unfolded (`true`), or only the first, as VS Code's (default). e.g., `true` where
    /// there are few references to each document, as of a pointer.
    #[inline]
    #[must_use]
    pub fn unfolded(mut self, all: bool) -> Self {
        for (index, group) in self.groups.iter_mut().enumerate() {
            group.open = all || index == 0;
        }
        self
    }

    /// The next reference (`forward`) or the previous one, around the ends, as the one selected now.
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
        let group = self.group_of(next);
        self.groups[group].open = true;
        self.put_cursor(Item::Row(next));
        self.current_place()
    }

    /// The editor's line it opens under, and how tall it is. `cursor_line`: where the editor's cursor
    /// is, read the first time.
    pub(crate) fn gap(&mut self, cursor_line: impl FnOnce() -> usize) -> (usize, f32) {
        (*self.anchor.get_or_insert_with(cursor_line), self.height)
    }

    /// The `(document, line)` of the reference selected.
    fn current_place(&self) -> Option<(&str, usize)> {
        let row = self.rows.get(self.current?)?;
        Some((&row.document, row.line))
    }

    /// The group of row `row`.
    fn group_of(&self, row: usize) -> usize {
        self.groups.iter().position(|group| group.rows.contains(&row)).unwrap_or_default()
    }

    /// The lines of the list, as shown: each document, and the rows of the unfolded ones.
    fn items(&self) -> impl Iterator<Item = Item> + '_ {
        self.groups.iter().enumerate().flat_map(|(index, group)| {
            let rows = if group.open { group.rows.clone() } else { 0..0 };
            std::iter::once(Item::Group(index)).chain(rows.map(Item::Row))
        })
    }

    /// The keys are on `item` now; a row is the one selected.
    const fn put_cursor(&mut self, item: Item) {
        self.cursor = Some(item);
        self.moved = true;
        if let Item::Row(row) = item {
            self.current = Some(row);
            self.stale_preview = true;
        }
    }

    /// The next (`forward`) or the previous line of the list, stopping at the ends.
    fn move_cursor(&mut self, forward: bool) {
        let items: Vec<Item> = self.items().collect();
        let at = self.cursor.and_then(|cursor| items.iter().position(|&item| item == cursor)).unwrap_or_default();
        let next = if forward { (at + 1).min(items.len().saturating_sub(1)) } else { at.saturating_sub(1) };
        if let Some(&item) = items.get(next) {
            self.put_cursor(item);
        }
    }

    /// The first (`!last`) or the last line of the list.
    fn cursor_to_end(&mut self, last: bool) {
        let item = if last { self.items().last() } else { self.items().next() };
        if let Some(item) = item {
            self.put_cursor(item);
        }
    }

    /// Folds (`Some(false)`), unfolds (`Some(true)`) or toggles (`None`) the document at the cursor.
    /// Folded, the cursor goes on its document from one of its rows.
    fn fold(&mut self, open: Option<bool>) {
        let group = match self.cursor {
            Some(Item::Group(group)) => group,
            Some(Item::Row(row)) => self.group_of(row),
            None => return,
        };
        let open = open.unwrap_or(!self.groups[group].open);
        self.groups[group].open = open;
        if !open {
            self.put_cursor(Item::Group(group));
        }
    }

    /// Draws the peek in the editor's gap at `top`, as wide as the editor's view. Returns what was
    /// asked; `list` is `None` once it has closed.
    pub(crate) fn show_inline(
        ui: &mut egui::Ui,
        list: &mut Option<Self>,
        input: &PeekInput<'_>,
        top: f32,
    ) -> Option<PeekAsk> {
        let references = list.as_mut()?;
        references.read_snippets(input.texts);
        references.load_preview(input.texts);

        let view = ui.clip_rect();
        let rect = egui::Rect::from_x_y_ranges(view.x_range(), top..=top + references.height);
        if references.take_keyboard {
            ui.scroll_to_rect(rect, None); // just opened: shown whole
        }
        let builder = egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min));
        let asked = ui
            .scope_builder(builder, |ui| {
                ui.set_clip_rect(rect.intersect(view));
                references.peek_ui(ui, input.language)
            })
            .inner;
        // The wheel over it is its own: what its preview and list did not take (at their ends, or
        // nothing to scroll) would scroll the editor around it.
        if ui.rect_contains_pointer(rect) {
            ui.input_mut(|input| input.smooth_scroll_delta = egui::Vec2::ZERO);
        }
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
        if !std::mem::take(&mut self.stale_preview) {
            return;
        }
        let Some((document, line)) = self.current_place() else { return };
        if *self.preview.document != *document {
            let text = texts(document).unwrap_or_default();
            self.preview = Preview { document: document.into(), text, ..Preview::default() };
        }
        self.preview.line = Some(line);
    }

    /// The accent lines and the title, then the preview and the list side by side.
    fn peek_ui(&mut self, ui: &mut egui::Ui, language: &dyn Language) -> Option<PeekAsk> {
        let rect = ui.max_rect();
        let accent = egui::Stroke::new(ACCENT_LINE, ui.visuals().selection.stroke.color);
        ui.painter().rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
        ui.painter().hline(rect.x_range(), rect.top(), accent);
        ui.painter().hline(rect.x_range(), rect.bottom(), accent);
        ui.add_space(ACCENT_LINE + 2.0);

        let closed = self.title_ui(ui);
        let height = ui.available_height() - ACCENT_LINE;
        let preview_width = ui.available_width() * PREVIEW_SHARE;
        // Each side top down: inside the row, they would lay out their lines side by side.
        let column = egui::Layout::top_down(egui::Align::Min);
        let (from_preview, from_list) = ui
            .horizontal_top(|ui| {
                let preview_size = egui::vec2(preview_width, height);
                let from_preview = ui.allocate_ui_with_layout(preview_size, column, |ui| self.preview_ui(ui, language));
                ui.separator();
                let list_size = egui::vec2(ui.available_width(), height);
                let from_list = ui.allocate_ui_with_layout(list_size, column, |ui| self.list_ui(ui));
                (from_preview.inner, from_list.inner)
            })
            .inner;
        self.moved = false;
        if closed { Some(PeekAsk::Close) } else { from_list.or(from_preview) }
    }

    /// `#0105  hkbClipGenerator — References (3) ............ ✖`, of the reference selected.
    /// Returns whether `✖` was clicked.
    fn title_ui(&self, ui: &mut egui::Ui) -> bool {
        let mut closed = false;
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            if let Some(row) = self.current.and_then(|at| self.rows.get(at)) {
                ui.strong(&*row.document);
                ui.weak(&*row.detail);
            }
            ui.weak(format!("— {}: References ({})", self.symbol, self.rows.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(CLOSE_MARGIN);
                // Its background shows on hover only, as VS Code's.
                let button = egui::Button::new("✖").frame_when_inactive(false).min_size(egui::Vec2::splat(CLOSE_SIZE));
                let close = ui.add(button).on_hover_cursor(egui::CursorIcon::PointingHand);
                closed = close.clicked();
            });
        });
        closed
    }

    /// The document of the selected row, read only, its cursor at the row's line.
    /// Returns [`PeekAsk::Open`] at the cursor's line on a double click, or on `Enter` while it has
    /// the keyboard.
    fn preview_ui(&mut self, ui: &mut egui::Ui, language: &dyn Language) -> Option<PeekAsk> {
        let id_salt = ("references_peek_preview", &*self.preview.document);
        let editor_id = crate::editor_id(id_salt);
        if let Some(line) = self.preview.line.take() {
            let at = crate::text_pos::LinePos { line, column: 0 }.to_char_index(&self.preview.text);
            crate::place_cursor(ui.ctx(), editor_id, at);
        }
        // Taken before the editor reads it.
        let enter = ui.memory(|memory| memory.has_focus(editor_id))
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));

        let scroll = egui::ScrollArea::both().id_salt("references_peek_preview_scroll").auto_shrink(false);
        let clicked = scroll.show(ui, |ui| {
            let editor = CodeEditor::new(id_salt, &mut self.preview.text).language(language).read_only(true);
            let output = editor.show(ui, &mut self.preview.state);
            output.cursor.filter(|_| enter || output.response.double_clicked()).map(|cursor| cursor.line)
        });
        clicked.inner.map(|line| PeekAsk::Open(self.preview.document.to_string(), line))
    }

    /// The references by document, the cursor's line marked; and the keys while it has the keyboard.
    fn list_ui(&mut self, ui: &mut egui::Ui) -> Option<PeekAsk> {
        let area = ui.interact(
            ui.max_rect(),
            ui.scope_id().with("references_peek_list"),
            egui::Sense::focusable_noninteractive(),
        );
        if std::mem::take(&mut self.take_keyboard) {
            area.request_focus();
        }
        if area.has_focus() {
            // Its keys, not egui's: egui drops the keyboard on `Esc` and moves it to the next widget
            // on the arrows, before the list reads them.
            let filter = egui::EventFilter { horizontal_arrows: true, vertical_arrows: true, escape: true, tab: false };
            ui.memory_mut(|memory| memory.set_focus_lock_filter(area.id, filter));
        }
        let mut asked = if area.has_focus() { self.keys(ui) } else { None };
        if self.stale_preview {
            ui.ctx().request_repaint(); // the preview follows on the next frame
        }

        let mut clicked = None;
        // Each line as wide as the list, its text at the left (`add_sized` would center it).
        let lines = egui::Layout::top_down_justified(egui::Align::Min);
        let scroll = egui::ScrollArea::vertical().id_salt("references_peek_list_scroll").auto_shrink(false);
        scroll.show(ui, |ui| {
            ui.with_layout(lines, |ui| {
                let items: Vec<Item> = self.items().collect();
                for item in items {
                    let on_cursor = self.cursor == Some(item);
                    let response = match item {
                        Item::Group(group) => self.group_line(ui, group, on_cursor),
                        Item::Row(row) => reference_line(ui, &self.rows[row], on_cursor),
                    };
                    if on_cursor && self.moved {
                        response.scroll_to_me(None);
                    }
                    if response.clicked() {
                        clicked = Some(item);
                    }
                    if let (Item::Row(row), true) = (item, response.double_clicked()) {
                        let row = &self.rows[row];
                        asked = Some(PeekAsk::Open(row.document.to_string(), row.line));
                    }
                }
            })
        });
        if let Some(item) = clicked {
            self.put_cursor(item);
            if matches!(item, Item::Group(_)) {
                self.fold(None);
            }
            area.request_focus();
        }
        asked
    }

    /// `▾ #0105  hkbClipGenerator ........ (2)`: a document, unfolded or not, with its count.
    fn group_line(&self, ui: &mut egui::Ui, group: usize, on_cursor: bool) -> egui::Response {
        let Group { rows, open } = &self.groups[group];
        let row = &self.rows[rows.start];
        let fold = if *open { "▾" } else { "▸" };
        let text = format!("{fold} {}  {}", row.document, row.detail);
        let response = ui.add(egui::Button::selectable(on_cursor, text).truncate());
        count_badge(ui, response.rect, rows.len());
        response
    }

    /// The keys of the list. Returns what was asked.
    fn keys(&mut self, ui: &egui::Ui) -> Option<PeekAsk> {
        use egui::{Key, Modifiers};
        let pressed = |key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));

        if pressed(Key::ArrowDown) || pressed(Key::J) {
            self.move_cursor(true);
        }
        if pressed(Key::ArrowUp) || pressed(Key::K) {
            self.move_cursor(false);
        }
        // `Shift+G` first: egui's `consume_key` takes it for a plain `g` too.
        if ui.input_mut(|input| input.consume_key(Modifiers::SHIFT, Key::G)) {
            self.cursor_to_end(true);
        }
        if pressed(Key::G) {
            self.cursor_to_end(false);
        }
        if pressed(Key::O) || pressed(Key::Space) {
            self.fold(None);
        }
        if pressed(Key::ArrowLeft) || pressed(Key::H) {
            self.fold(Some(false));
        }
        if pressed(Key::ArrowRight) || pressed(Key::L) {
            self.fold(Some(true));
        }
        if pressed(Key::Escape) {
            return Some(PeekAsk::Close);
        }
        if pressed(Key::Enter) {
            if matches!(self.cursor, Some(Item::Group(_))) {
                self.fold(None);
                return None;
            }
            let (document, line) = self.current_place()?;
            return Some(PeekAsk::Open(document.to_owned(), line));
        }
        None
    }
}

/// `    12  <hkparam name="id">#0106</hkparam>`, on the cursor or not. People count lines from 1.
fn reference_line(ui: &mut egui::Ui, row: &Row, on_cursor: bool) -> egui::Response {
    let snippet = row.snippet.as_deref().unwrap_or_default();
    let text = egui::RichText::new(format!("    {:>4}  {snippet}", row.line + 1)).monospace();
    ui.add(egui::Button::selectable(on_cursor, text).truncate())
}

/// The count of a document's references, in a round badge at the right of its line, as VS Code's.
fn count_badge(ui: &egui::Ui, line: egui::Rect, count: usize) {
    let font = egui::TextStyle::Small.resolve(ui.style());
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(count.to_string(), font, ui.visuals().strong_text_color());
    let size = egui::vec2(galley.size().x.max(galley.size().y) + 6.0, galley.size().y + 2.0);
    let badge = egui::Rect::from_center_size(egui::pos2(line.right() - size.x / 2.0 - 4.0, line.center().y), size);
    painter.rect_filled(badge, size.y / 2.0, ui.visuals().selection.bg_fill);
    painter.galley(badge.center() - galley.size() / 2.0, galley, egui::Color32::PLACEHOLDER);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Location;

    fn located(places: &[(&str, usize)]) -> ReferenceList {
        let locations =
            places.iter().map(|&(document, line)| Location { document: document.into(), line, detail: "".into() });
        ReferenceList::from(References { symbol: "x".into(), locations: locations.collect() })
    }

    fn list(lines: &[usize]) -> ReferenceList {
        located(&lines.iter().map(|&line| ("#a", line)).collect::<Vec<_>>())
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

    /// By document, the first unfolded as VS Code's; a step into a folded one unfolds it.
    #[test]
    fn the_list_is_by_document_folded_but_the_first() {
        let mut refs = located(&[("#a", 1), ("#a", 5), ("#b", 2)]);
        let shown: Vec<Item> = refs.items().collect();
        assert_eq!(shown, [Item::Group(0), Item::Row(0), Item::Row(1), Item::Group(1)]);

        refs.step(false);
        assert_eq!(refs.items().last(), Some(Item::Row(2)), "unfolded by the step");

        let all = located(&[("#a", 1), ("#b", 2)]).unfolded(true);
        let shown: Vec<Item> = all.items().collect();
        assert_eq!(shown, [Item::Group(0), Item::Row(0), Item::Group(1), Item::Row(1)], "all unfolded");
    }

    /// `j` `k` go line by line, stopping at the ends; `o` folds the document at the cursor, which
    /// goes on it.
    #[test]
    fn the_cursor_goes_line_by_line_and_folds() {
        let mut refs = located(&[("#a", 1), ("#a", 5), ("#b", 2)]);
        refs.move_cursor(false);
        refs.move_cursor(false);
        assert_eq!(refs.cursor, Some(Item::Group(0)), "the top");

        refs.move_cursor(true);
        refs.fold(None);
        assert_eq!(refs.cursor, Some(Item::Group(0)));
        assert_eq!(refs.items().collect::<Vec<_>>(), [Item::Group(0), Item::Group(1)]);

        refs.move_cursor(true);
        refs.fold(Some(true));
        refs.move_cursor(true);
        assert_eq!(refs.current_place(), Some(("#b", 2)));

        refs.cursor_to_end(false);
        assert_eq!(refs.cursor, Some(Item::Group(0)), "`g`: the top");
        refs.cursor_to_end(true);
        assert_eq!(refs.cursor, Some(Item::Row(2)), "`G`: the bottom");
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

    /// The preview follows the cursor to another reference, after a frame that drew it.
    #[test]
    fn the_preview_follows_the_cursor() {
        let mut refs = located(&[("#a", 1), ("#a", 5)]);
        let texts = |document: &str| Some(document.to_owned());
        refs.load_preview(&texts);
        refs.moved = false; // the frame ended

        refs.move_cursor(true);
        refs.moved = false; // the keys come after the preview, in the same frame
        refs.load_preview(&texts);
        assert_eq!(refs.preview.line, Some(5));
    }
}
