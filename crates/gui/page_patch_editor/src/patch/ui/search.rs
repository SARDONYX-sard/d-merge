//! The side bar's search, drawn as VS Code's: the field, a summary, and the results under their
//! object, in the editor's colors with each match highlighted. The logic is in [`crate::patch::search`].
//!
//! ```text
//! [$eventID[moveStart]$          ] [.*]
//! 3 results in 2 objects
//! ────────────────────────────────────────────  <- a line and a tinted band start each object
//! ▼ #0106  hkbModifierList  GetUpModList      2   <- a click folds it
//!     <hkparam name="id">$eventID[moveStart]$</hkparam>   <- XML colors, the match on a tint;
//!     ...                                                     a click opens that line
//! ────────────────────────────────────────────
//! ▶ #0240  hkbStateMachine                     1
//! ```

use egui::{Color32, text::LayoutJob};
use egui_code_editor::{Language as _, languages::Xml};
use egui_widgets::shadcn::{Button, TextEdit};
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{
    PatchTool,
    search::{Hit, SearchRow},
};

/// Pixels the hits are indented by, under their object.
const HIT_INDENT: f32 = 18.0;

impl PatchTool {
    pub(in crate::patch) fn search_panel(&mut self, ui: &mut egui::Ui, i18n: &I18nMap) {
        // One row, right to left: `.*` takes its width first, the field gets the rest. (Without the
        // row, the layout takes all the height left, with the field in its middle and no room below.)
        let field = ui
            .horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let regex = Button::selectable(self.search.regex, ".*").small();
                    if ui.add(regex).on_hover_text(i18n.t(I18nKey::SearchRegexHover)).clicked() {
                        self.search.regex = !self.search.regex;
                    }
                    let field = TextEdit::singleline(&mut self.search.query)
                        .hint_text(i18n.t(I18nKey::NemesisSearchHint))
                        .desired_width(ui.available_width());
                    ui.add(field)
                })
                .inner
            })
            .inner;
        if std::mem::take(&mut self.focus_search) {
            field.request_focus();
        }

        let generation = self.names().graph.generation();
        self.search.refresh(self.active, generation, self.workspace.behaviors[self.active].texts());
        if self.search.query.trim().is_empty() {
            return;
        }
        // `3 results in 2 objects`, as VS Code's line above the results.
        let more = if self.search.is_cut() { "+" } else { "" };
        let (hits, objects) = (self.search.hits.len(), self.search.object_count());
        let summary =
            format!("{hits}{more} {} {objects} {}", i18n.t(I18nKey::SearchResultsIn), i18n.t(I18nKey::SearchObjects));
        ui.weak(summary);

        let rows = self.search.rows();
        let (mut open, mut fold) = (None, None);
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 6.0;
        egui::ScrollArea::both().id_salt("nemesis_search").auto_shrink(false).show_rows(
            ui,
            row_height,
            rows.len(),
            |ui, range| {
                for row in &rows[range] {
                    match *row {
                        SearchRow::Object { first, count } => {
                            if self.object_row(ui, first, count, row_height) {
                                fold = Some(first);
                            }
                        }
                        SearchRow::Hit(at) => {
                            if hit_row(ui, &self.search.hits[at], row_height) {
                                open = Some(at);
                            }
                        }
                    }
                }
            },
        );
        if let Some(first) = fold {
            let object = self.search.hits[first].object.clone();
            self.search.toggle(&object);
        }
        if let Some(at) = open {
            let (object, line) = (self.search.hits[at].object.clone(), self.search.hits[at].line);
            let current = self.selected.clone().unwrap_or_default();
            self.open_location(&current, &object, line);
        }
    }

    /// `▼ #0106  hkbModifierList  GetUpModList      2` on a tinted band under a line. Returns whether it
    /// was clicked (to fold).
    fn object_row(&self, ui: &mut egui::Ui, first: usize, count: usize, height: f32) -> bool {
        let id = &*self.search.hits[first].object;
        let folded = self.search.folded.contains(id);
        let object = self.workspace.behaviors[self.active].object_text(id);
        let (class, name) = object.map_or(("", ""), |object| (object.class, object.name));

        let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click());
        let visuals = ui.visuals();
        let band = if response.hovered() { visuals.widgets.hovered.weak_bg_fill } else { visuals.faint_bg_color };
        ui.painter().rect_filled(rect, 0.0, band);
        ui.painter().hline(rect.x_range(), rect.top(), visuals.widgets.noninteractive.bg_stroke);

        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let mut job = LayoutJob::default();
        let mut add = |text: &str, color| job.append(text, 0.0, egui::TextFormat::simple(font.clone(), color));
        add(if folded { "▶ " } else { "▼ " }, visuals.weak_text_color());
        add(id, visuals.strong_text_color());
        add("  ", visuals.text_color());
        add(class, egui_widgets::markdown::class_color(visuals));
        add("  ", visuals.text_color());
        add(name, visuals.weak_text_color());
        let galley = ui.painter().layout_job(job);
        let text_at = egui::pos2(rect.left() + 4.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(text_at, galley, visuals.text_color());
        // The count at the right, as VS Code's badge.
        let count = count.to_string();
        let badge = egui::Align2::RIGHT_CENTER;
        let at = egui::pos2(rect.right() - 6.0, rect.center().y);
        ui.painter().text(at, badge, count, egui::FontId::proportional(11.0), visuals.weak_text_color());
        response.clicked()
    }
}

/// `    <hkparam name="id">$eventID[moveStart]$</hkparam>` in the editor's colors with its matches
/// tinted. Returns whether it was clicked (to open it).
fn hit_row(ui: &mut egui::Ui, hit: &Hit, height: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let job = colored_line(ui, &hit.text, &hit.matches);
    let galley = ui.painter().layout_job(job);
    let text_at = egui::pos2(rect.left() + HIT_INDENT, rect.center().y - galley.size().y / 2.0);
    ui.painter().with_clip_rect(rect).galley(text_at, galley, ui.visuals().text_color());
    response.on_hover_text(format!("{}:{}", hit.object, hit.line + 1)).clicked()
}

/// `text` as the editor colors XML, with the byte ranges `matches` on VS Code's tint of a match.
fn colored_line(ui: &egui::Ui, text: &str, matches: &[std::ops::Range<usize>]) -> LayoutJob {
    let visuals = ui.visuals();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    // The highlight's ranges are of chars; the matches', of bytes.
    let colors = Xml.highlight(text, visuals);
    let color_at = |char_at: usize| {
        colors
            .iter()
            .find(|(range, _)| range.contains(&char_at))
            .map_or_else(|| visuals.text_color(), |(_, color)| *color)
    };
    let tint = visuals.selection.bg_fill.gamma_multiply(0.7);
    let format_at = |char_at: usize, byte_at: usize| {
        let background = if matches.iter().any(|range| range.contains(&byte_at)) { tint } else { Color32::TRANSPARENT };
        egui::TextFormat { background, ..egui::TextFormat::simple(font.clone(), color_at(char_at)) }
    };
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
