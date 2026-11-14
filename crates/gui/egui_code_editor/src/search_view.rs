//! Searching every document at once, as VS Code's `Ctrl+Shift+F`: the field, a summary, and the
//! matching lines under their document, in the language's colors with each match tinted.
//!
//! ```text
//! [$eventID[moveStart]$          ] [.*]
//! 3 results in 2 objects
//! ────────────────────────────────────────────  <- a line and a tinted band start each document
//! ▼ #0106  hkbModifierList  GetUpModList      2   <- the host's label; a click folds it
//!     <hkparam name="id">$eventID[moveStart]$</hkparam>   <- a click opens that line
//! ```
//! - Text ignoring ASCII case, or with `.*` on a regex ignoring case (e.g. `#0\d{3}`). A regex not
//!   finished yet finds nothing.
//! - It searches again only when the query or the documents' `version` changed, and keeps at most
//!   [`SearchView::MAX_HITS`] lines: a short query matches nearly every line.
//!
//! ```text
//! let mut search = SearchView::default();                      // kept by the host
//! search.refresh(version, docs.iter().map(|d| (d.id, d.text)));
//! let open = search.show(ui, &labels, &Xml, |job, id, visuals| { /* append the label of `id` */ });
//! if let Some((id, line)) = open { /* open document `id` at `line` */ }
//! ```

use std::{
    hash::{Hash, Hasher},
    ops::Range,
};

use egui::{Color32, text::LayoutJob};

use crate::Language;

/// One matching line. Kept across frames, so it owns its text.
#[derive(Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub document: Box<str>,
    /// 0-based.
    pub line: usize,
    /// Without its indent.
    pub text: Box<str>,
    /// Where `text` matches (byte ranges), to show them.
    pub matches: Box<[Range<usize>]>,
}

/// The words of the view, in the host's language. e.g. `{ hint: "Search", regex_hover: "Use
/// Regular Expression", results_in: "results in", documents: "files" }`
#[derive(Debug, Clone, Copy)]
pub struct SearchLabels<'a> {
    pub hint: &'a str,
    pub regex_hover: &'a str,
    /// `3 results in 2 files`: `results in`.
    pub results_in: &'a str,
    /// `3 results in 2 files`: `files`.
    pub documents: &'a str,
}

/// See the module docs.
#[derive(Debug, Default)]
pub struct SearchView {
    pub query: String,
    /// `.*`: `query` is a regex.
    pub regex: bool,
    pub hits: Vec<SearchHit>,
    /// The documents whose hits are folded away.
    folded: std::collections::BTreeSet<Box<str>>,
    /// What `hits` was found for: the query, `regex`, and the documents' version (hashed).
    searched: Option<(String, bool, u64)>,
    /// The field takes the keyboard on the next [`Self::show`].
    focus: bool,
}

/// A row of the results: a document (`hits[first]`'s, with `count` hits), or `hits[index]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Document { first: usize, count: usize },
    Hit(usize),
}

/// Pixels the hits are indented by, under their document.
const HIT_INDENT: f32 = 18.0;

impl SearchView {
    /// The most hits kept.
    pub const MAX_HITS: usize = 1000;

    /// The field takes the keyboard when next shown, e.g. on `Ctrl+Shift+F`.
    #[inline]
    pub const fn focus(&mut self) {
        self.focus = true;
    }

    /// Searches `documents` (`(id, text)`) again if the query or `version` (e.g. the project and its
    /// edit count) changed since the last time.
    pub fn refresh<'a>(&mut self, version: impl Hash, documents: impl Iterator<Item = (&'a str, &'a str)>) {
        let version = {
            let mut hasher = std::hash::DefaultHasher::new();
            version.hash(&mut hasher);
            hasher.finish()
        };
        let unchanged = self.searched.as_ref().is_some_and(|(query, regex, searched)| {
            *query == self.query && *regex == self.regex && *searched == version
        });
        if unchanged {
            return;
        }

        let query = self.query.trim();
        self.hits = if self.regex {
            // `(?i)`: as the text search, case does not matter.
            regex::RegexBuilder::new(query).case_insensitive(true).build().map_or_else(
                |_| Vec::new(),
                |regex| find(documents, query, |line| regex.find_iter(line).map(|found| found.range()).collect()),
            )
        } else {
            find(documents, query, |line| stdx::text::find_ignore_case(line, query).collect())
        };
        self.searched = Some((self.query.clone(), self.regex, version));
    }

    /// How many documents have hits.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.hits.chunk_by(|a, b| a.document == b.document).count()
    }

    /// Folds or unfolds the hits of `document`.
    pub fn toggle(&mut self, document: &str) {
        if !self.folded.remove(document) {
            self.folded.insert(document.into());
        }
    }

    /// Each document (with its count), then its hits unless folded.
    ///
    /// e.g. hits `#1:3`, `#1:9`, `#2:0` -> `#1 (2)`, `#1:3`, `#1:9`, `#2 (1)`, `#2:0`
    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::with_capacity(self.hits.len() + 16);
        let mut at = 0;
        for group in self.hits.chunk_by(|a, b| a.document == b.document) {
            rows.push(Row::Document { first: at, count: group.len() });
            if !self.folded.contains(&group[0].document) {
                rows.extend((at..at + group.len()).map(Row::Hit));
            }
            at += group.len();
        }
        rows
    }

    /// Draws the field and the results, the lines colored by `language`. `label` appends what a
    /// document's row says after `▼ ` (e.g. its id, then its kind), as the host names its documents.
    /// Returns the line a click opened: `(document, line)`.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        labels: &SearchLabels<'_>,
        language: &dyn Language,
        label: impl Fn(&mut LayoutJob, &str, &egui::Visuals),
    ) -> Option<(Box<str>, usize)> {
        self.field(ui, labels);
        if self.query.trim().is_empty() {
            return None;
        }

        // `3 results in 2 files`, as VS Code's line above the results.
        let more = if self.hits.len() >= Self::MAX_HITS { "+" } else { "" };
        ui.weak(format!(
            "{}{more} {} {} {}",
            self.hits.len(),
            labels.results_in,
            self.document_count(),
            labels.documents
        ));

        let (open, fold) = {
            let rows = self.rows();
            let height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
            let (mut open, mut fold) = (None, None);
            let scroll = egui::ScrollArea::both().id_salt("code_editor_search_view").auto_shrink(false);
            scroll.show_rows(ui, height, rows.len(), |ui, range| {
                for row in &rows[range] {
                    match *row {
                        Row::Document { first, count } => {
                            let document = &*self.hits[first].document;
                            let folded = self.folded.contains(document);
                            if document_row(ui, (document, folded, count), height, &label) {
                                fold = Some(first);
                            }
                        }
                        Row::Hit(at) => {
                            if hit_row(ui, &self.hits[at], height, language) {
                                open = Some(at);
                            }
                        }
                    }
                }
            });
            (open, fold)
        };

        if let Some(first) = fold {
            let document = self.hits[first].document.clone();
            self.toggle(&document);
        }

        open.map(|at| (self.hits[at].document.clone(), self.hits[at].line))
    }

    /// `[query] [.*]` in one row, right to left: `.*` takes its width first, the field the rest.
    /// (Without the row, the layout takes all the height left, with the field in its middle.)
    fn field(&mut self, ui: &mut egui::Ui, labels: &SearchLabels<'_>) {
        let field = ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let regex = egui_widgets::shadcn::Button::selectable(self.regex, ".*").small();
                if ui.add(regex).on_hover_text(labels.regex_hover).clicked() {
                    self.regex = !self.regex;
                }
                let field = egui_widgets::shadcn::TextEdit::singleline(&mut self.query)
                    .hint_text(labels.hint)
                    .desired_width(ui.available_width());
                ui.add(field)
            })
            .inner
        });
        if std::mem::take(&mut self.focus) {
            field.inner.request_focus();
        }
    }
}

/// The lines where `matches` finds something (none for an empty `query`), with what it found.
fn find<'a>(
    documents: impl Iterator<Item = (&'a str, &'a str)>,
    query: &str,
    matches: impl Fn(&str) -> Vec<Range<usize>>,
) -> Vec<SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut hits = Vec::new();
    for (document, text) in documents {
        for (line, text) in text.lines().enumerate() {
            let text = text.trim();
            let found = matches(text);
            if !found.is_empty() {
                hits.push(SearchHit { document: document.into(), line, text: text.into(), matches: found.into() });
                if hits.len() >= SearchView::MAX_HITS {
                    return hits;
                }
            }
        }
    }
    hits
}

/// `▼ <label>      2` on a tinted band under a line. Returns whether it was clicked (to fold).
fn document_row(
    ui: &mut egui::Ui,
    (document, folded, count): (&str, bool, usize),
    height: f32,
    label: &impl Fn(&mut LayoutJob, &str, &egui::Visuals),
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click());
    let visuals = ui.visuals();
    let band = if response.hovered() { visuals.widgets.hovered.weak_bg_fill } else { visuals.faint_bg_color };
    ui.painter().rect_filled(rect, 0.0, band);
    ui.painter().hline(rect.x_range(), rect.top(), visuals.widgets.noninteractive.bg_stroke);

    // `▼ ` and the host's label.
    {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let mut job = LayoutJob::default();
        let arrow = if folded { "▶ " } else { "▼ " };
        job.append(arrow, 0.0, egui::TextFormat::simple(font, visuals.weak_text_color()));
        label(&mut job, document, visuals);
        let galley = ui.painter().layout_job(job);
        let text_at = egui::pos2(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(text_at, galley, visuals.text_color());
    }

    // The count at the right, as VS Code's badge.
    {
        let at = egui::pos2(rect.right() - 6.0, rect.center().y);
        let font = egui::FontId::proportional(11.0);
        ui.painter().text(at, egui::Align2::RIGHT_CENTER, count.to_string(), font, visuals.weak_text_color());
    }

    response.clicked()
}

/// A matching line in the language's colors with its matches tinted. Returns whether it was clicked
/// (to open it).
fn hit_row(ui: &mut egui::Ui, hit: &SearchHit, height: f32, language: &dyn Language) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }

    let galley = ui.painter().layout_job(colored_line(ui, &hit.text, &hit.matches, language));
    let text_at = egui::pos2(rect.left() + HIT_INDENT, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(text_at, galley, ui.visuals().text_color());

    response.on_hover_text(format!("{}:{}", hit.document, hit.line + 1)).clicked()
}

/// `text` as `language` colors it, with the byte ranges `matches` on VS Code's tint of a match.
fn colored_line(ui: &egui::Ui, text: &str, matches: &[Range<usize>], language: &dyn Language) -> LayoutJob {
    // What a char looks like: the highlight's color (its ranges are of chars), on the tint if a match
    // (their ranges are of bytes) has it.
    let visuals = ui.visuals();
    let format_at = {
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let colors = language.highlight(text, visuals);
        let tint = visuals.selection.bg_fill.gamma_multiply(0.7);
        move |char_at: usize, byte_at: usize| {
            let color = colors
                .iter()
                .find(|(range, _)| range.contains(&char_at))
                .map_or_else(|| visuals.text_color(), |(_, color)| *color);
            let matched = matches.iter().any(|range| range.contains(&byte_at));
            let background = if matched { tint } else { Color32::TRANSPARENT };
            egui::TextFormat { background, ..egui::TextFormat::simple(font.clone(), color) }
        }
    };

    // Runs of chars that look the same, one section each.
    let mut job = LayoutJob::default();
    let mut run: Option<(usize, egui::TextFormat)> = None; // (start byte, format) of the run being added
    for (char_at, (byte_at, _)) in text.char_indices().enumerate() {
        let format = format_at(char_at, byte_at);
        match &run {
            Some((_, current)) if *current == format => {}
            _ => {
                if let Some((start, current)) = run.take() {
                    job.append(&text[start..byte_at], 0.0, current);
                }
                run = Some((byte_at, format));
            }
        }
    }
    if let Some((start, current)) = run {
        job.append(&text[start..], 0.0, current);
    }

    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_lines_ignoring_case() {
        let documents = [("#1", "a\n\t$eventID[moveStart]$\n"), ("#2", "$EVENTID[movestart]$"), ("#3", "b")];
        let mut search = SearchView { query: "$eventid[moveStart]$".to_owned(), ..SearchView::default() };
        search.refresh(1, documents.into_iter());
        assert_eq!(search.hits.len(), 2);
        let hit = SearchHit {
            document: "#1".into(),
            line: 1,
            text: "$eventID[moveStart]$".into(),
            matches: std::iter::once(0..20).collect(),
        };
        assert_eq!(search.hits[0], hit);
        assert_eq!(&*search.hits[1].document, "#2");
    }

    /// `.*`: the same query is a regex, ignoring case too; one not finished yet finds nothing.
    #[test]
    fn a_regex_finds_lines_too() {
        let documents = || [("#1", "<hkparam>#0106</hkparam>\n#M$0"), ("#2", "#12")];
        let mut search = SearchView { query: r"#\d{4}|#m\$".to_owned(), regex: true, ..SearchView::default() };
        search.refresh(1, documents().into_iter());
        assert_eq!(search.hits.iter().map(|hit| hit.line).collect::<Vec<_>>(), [0, 1]);

        search.query = "[0-".to_owned();
        search.refresh(1, documents().into_iter());
        assert!(search.hits.is_empty());
    }

    #[test]
    fn hits_are_shown_under_their_document_unless_folded() {
        let mut search = SearchView { query: "a".to_owned(), ..SearchView::default() };
        search.refresh(1, [("#1", "a\nb\na"), ("#2", "a")].into_iter());
        let document = |first, count| Row::Document { first, count };
        assert_eq!(search.rows(), [document(0, 2), Row::Hit(0), Row::Hit(1), document(2, 1), Row::Hit(2)]);
        assert_eq!(search.document_count(), 2);
        search.toggle("#1");
        assert_eq!(search.rows(), [document(0, 2), document(2, 1), Row::Hit(2)]);
    }

    #[test]
    fn searches_again_only_when_something_changed() {
        let mut search = SearchView { query: "a".to_owned(), ..SearchView::default() };
        search.refresh(1, std::iter::once(("#1", "a")));
        assert_eq!(search.hits.len(), 1);
        search.refresh(1, std::iter::once(("#1", "b"))); // nothing said that it changed
        assert_eq!(search.hits.len(), 1);
        search.refresh(2, std::iter::once(("#1", "b")));
        assert!(search.hits.is_empty());

        search.query.clear();
        search.refresh(2, std::iter::once(("#1", "b")));
        assert!(search.hits.is_empty());
    }
}
