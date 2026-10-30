//! Text layout for the editor: syntax colors + diagnostic underlines.
//!
//! egui's `TextEdit` asks a *layouter* to turn the text into a [`egui::Galley`] every frame.
//! We build a [`LayoutJob`] whose sections carry the colors from `hkanno_lsp`'s highlighter
//! and an underline for every diagnostic range.
//!
//! # Wrapping
//! Lines are **never wrapped**, so galley row `n` is always document line `n`.
//! Other parts of the editor (inlay hints, preview sync) rely on that.

use std::sync::Arc;

use egui::{Color32, Stroke, text::LayoutJob};
use hkanno_lsp::features::{
    diagnostics::{Diagnostic, Severity},
    highlight::{TokenKind, highlight_line},
};

/// Builds the galley for `text`.
///
/// - `semantic`: parser-based highlighting (LSP option "semantic tokens")
/// - `diagnostics`: ranges to underline (pass `&[]` when the option is off)
pub(super) fn layout(
    ui: &egui::Ui,
    text: &str,
    semantic: bool,
    diagnostics: &[Diagnostic],
) -> Arc<egui::Galley> {
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let dark = ui.visuals().dark_mode;
    let default_color = ui.visuals().text_color();

    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY;

    for (line_no, line) in text.split('\n').enumerate() {
        if line_no > 0 {
            job.append("\n", 0.0, format(&font_id, default_color, None));
        }

        // Per-char style; lines are short so this is simpler than merging ranges.
        // e.g., `0.5 animmotion` -> [Number x3, None, Identifier x10]
        let len = line.chars().count();
        let mut colors = vec![default_color; len];
        let mut underlines: Vec<Option<Color32>> = vec![None; len];

        for (span, kind) in highlight_line(line, semantic) {
            fill(&mut colors, span.start, span.end, token_color(kind, dark));
        }
        for diagnostic in diagnostics.iter().filter(|d| d.range.line == line_no) {
            let color = severity_color(ui.visuals(), diagnostic.severity);
            // An empty range (e.g., missing value at the line end) still gets one visible char.
            let span = diagnostic.range.span;
            let end = span.end.max(span.start + 1);
            fill(&mut underlines, span.start, end, Some(color));
        }

        append_runs(&mut job, line, &colors, &underlines, &font_id);
    }

    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// Color of a diagnostic severity (also used by the problems list).
pub(crate) const fn severity_color(visuals: &egui::Visuals, severity: Severity) -> Color32 {
    match severity {
        Severity::Error => visuals.error_fg_color,
        Severity::Warning => visuals.warn_fg_color,
    }
}

/// Theme color for each token kind (One Dark / One Light palette, like the former Monaco theme).
pub(super) const fn token_color(kind: TokenKind, dark: bool) -> Color32 {
    let (dark_rgb, light_rgb) = match kind {
        TokenKind::Comment => ((127, 132, 142), (160, 161, 167)),
        TokenKind::Number => ((209, 154, 102), (152, 104, 1)),
        TokenKind::String => ((152, 195, 121), (80, 161, 79)),
        TokenKind::Identifier => ((97, 175, 239), (64, 120, 242)),
        TokenKind::Variable => ((224, 108, 117), (228, 86, 73)),
        TokenKind::Function => ((229, 192, 123), (193, 132, 1)),
        TokenKind::Keyword => ((198, 120, 221), (166, 38, 164)),
        TokenKind::Operator => ((86, 182, 194), (1, 132, 188)),
    };
    let (r, g, b) = if dark { dark_rgb } else { light_rgb };
    Color32::from_rgb(r, g, b)
}

fn fill<T: Copy>(slots: &mut [T], start: usize, end: usize, value: T) {
    let end = end.min(slots.len());
    if start < end {
        slots[start..end].fill(value);
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
