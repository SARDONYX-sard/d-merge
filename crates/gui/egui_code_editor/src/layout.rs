//! Text layout: syntax colors + diagnostic underlines.
//!
//! egui's `TextEdit` asks a *layouter* to turn the text into a [`egui::Galley`] every frame.
//! We build a [`LayoutJob`] whose sections carry the colors from [`Language::highlight`]
//! and an underline for every [`Diagnostic`].
//!
//! # Wrapping
//! Lines are **never wrapped**, so galley row `n` is always document line `n`.
//! Inlay hints and line highlighting rely on that.

use std::sync::Arc;

use egui::{Color32, Stroke, text::LayoutJob};

use super::language::{Diagnostic, Language};

/// Builds the galley for `text`.
pub(super) fn layout(
    ui: &egui::Ui,
    text: &str,
    language: &dyn Language,
    diagnostics: &[Diagnostic],
) -> Arc<egui::Galley> {
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let visuals = ui.visuals();
    let default_color = visuals.text_color();

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

        for (columns, color) in language.highlight(line, visuals) {
            fill(&mut colors, columns, color);
        }
        for diagnostic in diagnostics.iter().filter(|d| d.line == line_no) {
            // An empty range (e.g., a missing value at the line end) still gets one visible char.
            let start = diagnostic.columns.start;
            let end = diagnostic.columns.end.max(start + 1);
            fill(&mut underlines, start..end, Some(diagnostic.severity.color(visuals)));
        }

        append_runs(&mut job, line, &colors, &underlines, &font_id);
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

    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;
    append_runs(&mut job, line, &colors, &underlines, &font_id);
    job
}

fn fill<T: Copy>(slots: &mut [T], range: std::ops::Range<usize>, value: T) {
    let end = range.end.min(slots.len());
    if range.start < end {
        slots[range.start..end].fill(value);
    }
}

/// Emits one section per run of chars that share the same color and underline.
fn append_runs(
    job: &mut LayoutJob,
    line: &str,
    colors: &[Color32],
    underlines: &[Option<Color32>],
    font_id: &egui::FontId,
) {
    // Byte offset of every char, plus the end, so runs can be sliced from `line`.
    let mut bytes: Vec<usize> = line.char_indices().map(|(i, _)| i).collect();
    bytes.push(line.len());

    let mut run_start = 0;
    for col in 1..=colors.len() {
        let run_ends = col == colors.len()
            || colors[col] != colors[run_start]
            || underlines[col] != underlines[run_start];
        if run_ends {
            let text = &line[bytes[run_start]..bytes[col]];
            job.append(text, 0.0, format(font_id, colors[run_start], underlines[run_start]));
            run_start = col;
        }
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
