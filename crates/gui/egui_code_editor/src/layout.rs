//! Text layout: syntax colors + diagnostic underlines.
//!
//! egui's `TextEdit` asks a *layouter* to turn the text into a [`egui::Galley`] every frame.
//! We build a [`LayoutJob`] whose sections carry the colors from [`Language::highlight`],
//! an underline for every [`Diagnostic`] and a background for every search match.
//!
//! # Wrapping
//! Lines are **never wrapped**, so galley row `n` is always document line `n`.
//! Inlay hints and line highlighting rely on that.

use std::sync::Arc;

use egui::{Color32, Stroke, text::LayoutJob};

use super::{
    language::{Diagnostic, Language},
    search::Matcher,
};

/// Builds the galley for `text`.
pub(super) fn layout(
    ui: &egui::Ui,
    text: &str,
    language: &dyn Language,
    diagnostics: &[Diagnostic],
    search: Option<&Matcher>,
) -> Arc<egui::Galley> {
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let default_color = visuals.text_color();
    let match_bg = search_match_color(visuals);

    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;

    for (line_no, line) in text.split('\n').enumerate() {
        if line_no > 0 {
            job.append("\n", 0.0, format(&font_id, default_color, None));
        }

        // Per-char style; lines are short so this is simpler than merging ranges.
        // e.g., `0.5 animmotion` -> [orange x3, default, blue x10]
        let len = line.chars().count();
        let mut colors = vec![default_color; len];
        let mut underlines: Vec<Option<Color32>> = vec![None; len];
        let mut backgrounds = vec![Color32::TRANSPARENT; len];

        for (columns, color) in language.highlight(line, visuals) {
            fill(&mut colors, columns, color);
        }
        for diagnostic in diagnostics.iter().filter(|d| d.line == line_no) {
            // An empty range (e.g., a missing value at the line end) still gets one visible char.
            let start = diagnostic.columns.start;
            let end = diagnostic.columns.end.max(start + 1);
            fill(&mut underlines, start..end, Some(diagnostic.severity.color(visuals)));
        }
        // e.g., Vim `/foo`: every `foo` gets a background.
        for columns in search.map(|m| m.find_all(line)).unwrap_or_default() {
            fill(&mut backgrounds, columns, match_bg);
        }

        append_runs(
            &mut job,
            line,
            Runs { colors: &colors, underlines: &underlines, backgrounds: &backgrounds },
            &font_id,
        );
    }

    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// One highlighted line, for drawing code outside the editor.
///
/// e.g., a read-only preview that only lays out its visible rows:
/// `ui.label(code_editor::line_job(ui, line, &Xml))`
pub fn line_job(ui: &egui::Ui, line: &str, language: &dyn Language) -> LayoutJob {
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();

    let mut colors = vec![visuals.text_color(); line.chars().count()];
    for (columns, color) in language.highlight(line, visuals) {
        fill(&mut colors, columns, color);
    }
    let underlines = vec![None; colors.len()];
    let backgrounds = vec![Color32::TRANSPARENT; colors.len()];

    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    append_runs(
        &mut job,
        line,
        Runs { colors: &colors, underlines: &underlines, backgrounds: &backgrounds },
        &font_id,
    );
    job
}

fn fill<T: Copy>(slots: &mut [T], range: std::ops::Range<usize>, value: T) {
    let end = range.end.min(slots.len());
    if range.start < end {
        slots[range.start..end].fill(value);
    }
}

/// Per-char styles of one line (all the same length: one entry per char).
struct Runs<'a> {
    colors: &'a [Color32],
    underlines: &'a [Option<Color32>],
    backgrounds: &'a [Color32],
}

impl Runs<'_> {
    /// The style of char `i`.
    const fn style(&self, i: usize) -> (Color32, Option<Color32>, Color32) {
        (self.colors[i], self.underlines[i], self.backgrounds[i])
    }
}

/// Emits one section per run of chars that share the same style.
fn append_runs(job: &mut LayoutJob, line: &str, runs: Runs<'_>, font_id: &egui::FontId) {
    // Byte offset of every char, plus the end, so runs can be sliced from `line`.
    let mut bytes: Vec<usize> = line.char_indices().map(|(i, _)| i).collect();
    bytes.push(line.len());

    let len = runs.colors.len();
    let mut run_start = 0;
    for col in 1..=len {
        if col == len || runs.style(col) != runs.style(run_start) {
            let (color, underline, background) = runs.style(run_start);
            let mut format = format(font_id, color, underline);
            format.background = background;
            job.append(&line[bytes[run_start]..bytes[col]], 0.0, format);
            run_start = col;
        }
    }
}

/// Background of search matches (like Vim's `Search` highlight group).
fn search_match_color(visuals: &egui::Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgba_unmultiplied(230, 180, 60, 90)
    } else {
        Color32::from_rgba_unmultiplied(255, 210, 60, 150)
    }
}

fn format(font_id: &egui::FontId, color: Color32, underline: Option<Color32>) -> egui::TextFormat {
    egui::TextFormat {
        font_id: font_id.clone(),
        color,
        underline: underline.map_or(Stroke::NONE, |c| Stroke::new(1.5_f32, c)),
        ..Default::default()
    }
}
