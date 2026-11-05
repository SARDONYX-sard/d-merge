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
    inline_diff::{InlineDiff, row_shifts},
    language::{Diagnostic, Language},
    search::Matcher,
};

/// The last galley, to not rebuild it while only the cursor moves.
///
/// Building one means highlighting every line of the text, per char. `TextEdit` asks for it every
/// frame, so without this, holding Vim `j` in a document of a few thousand lines lags behind the
/// keys. Part of [`crate::CodeEditorState`].
#[derive(Debug, Default)]
pub(super) struct LayoutCache {
    key: u64,
    galley: Option<Arc<egui::Galley>>,
}

/// [`layout`], reused while everything it depends on is the same.
pub(super) fn cached(
    cache: &mut LayoutCache,
    ui: &egui::Ui,
    text: &str,
    language: &dyn Language,
    diagnostics: &[Diagnostic],
    search: Option<&Matcher>,
    inline_diff: Option<&InlineDiff>,
) -> Arc<egui::Galley> {
    use std::hash::{Hash, Hasher};

    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    // The glyph width changes with the fonts (a galley does not get invalidated by `set_fonts`).
    let glyph_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font_id, '0'));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (text, diagnostics, search, inline_diff).hash(&mut hasher);
    (font_id.size.to_bits(), glyph_width.to_bits(), ui.pixels_per_point().to_bits()).hash(&mut hasher);
    (ui.visuals().dark_mode, ui.visuals().text_color()).hash(&mut hasher);
    let key = hasher.finish();

    if cache.key == key
        && let Some(galley) = &cache.galley
    {
        return Arc::clone(galley);
    }
    let mut galley = layout(ui, text, language, diagnostics, search);
    if let Some(diff) = inline_diff {
        make_room_for_removed_lines(&mut galley, diff);
    }
    *cache = LayoutCache { key, galley: Some(Arc::clone(&galley)) };
    galley
}

/// Moves rows down to leave gaps for the old lines; see [`crate::inline_diff`].
fn make_room_for_removed_lines(galley: &mut Arc<egui::Galley>, diff: &InlineDiff) {
    if diff.hunks.iter().all(|hunk| hunk.old_lines.is_empty()) {
        return;
    }
    let galley = Arc::make_mut(galley);
    let line_height = galley.rows.first().map_or(0.0, |row| row.rect().height());
    let (shifts, total) = row_shifts(galley.rows.len(), &diff.hunks);
    #[expect(clippy::cast_precision_loss, reason = "line counts are small")]
    let pixels = |lines: usize| lines as f32 * line_height;
    for (row, shift) in galley.rows.iter_mut().zip(shifts) {
        row.pos.y += pixels(shift);
    }
    galley.rect.max.y += pixels(total);
    galley.mesh_bounds.max.y += pixels(total);
}

/// Builds the galley for `text`.
fn layout(
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

    // Once per line of each edit: the same three buffers, and the problems found by their line instead
    // of going through all of them for each.
    let (mut colors, mut underlines, mut backgrounds) = (Vec::new(), Vec::new(), Vec::new());
    let mut by_line: Vec<&Diagnostic> = diagnostics.iter().collect();
    by_line.sort_by_key(|diagnostic| diagnostic.line);

    for (line_no, line) in text.split('\n').enumerate() {
        if line_no > 0 {
            job.append("\n", 0.0, format(&font_id, default_color, None));
        }

        // Per-char style; lines are short so this is simpler than merging ranges.
        // e.g., `0.5 animmotion` -> [orange x3, default, blue x10]
        let len = line.chars().count();
        colors.clear();
        colors.resize(len, default_color);
        underlines.clear();
        underlines.resize(len, None);
        backgrounds.clear();
        backgrounds.resize(len, Color32::TRANSPARENT);

        for (columns, color) in language.highlight(line, visuals) {
            fill(&mut colors, columns, color);
        }
        let first = by_line.partition_point(|diagnostic| diagnostic.line < line_no);
        for diagnostic in by_line[first..].iter().take_while(|diagnostic| diagnostic.line == line_no) {
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
    append_runs(&mut job, line, Runs { colors: &colors, underlines: &underlines, backgrounds: &backgrounds }, &font_id);
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

#[cfg(test)]
mod tests {
    use egui::text::CCursor;

    use super::*;
    use crate::inline_diff::DiffHunk;

    struct Plain;
    impl Language for Plain {}

    /// Two old lines above line 1: lines 1 and 2 move down, and positions still map to chars.
    #[test]
    fn the_gap_for_old_lines_keeps_cursor_positions_right() {
        egui::__run_test_ui(|ui| {
            let text = "aa\nbb\ncc";
            let hunk = DiffHunk { new_lines: 1..1, old_lines: vec![String::new(), String::new()], changes: Vec::new() };
            let diff = InlineDiff { hunks: vec![hunk] };

            let plain = cached(&mut LayoutCache::default(), ui, text, &Plain, &[], None, None);
            let shifted = cached(&mut LayoutCache::default(), ui, text, &Plain, &[], None, Some(&diff));
            let height = plain.rows[0].rect().height();
            let top = |galley: &egui::Galley, row: usize| galley.rows[row].rect().top();

            assert!((top(&shifted, 0) - top(&plain, 0)).abs() < 0.01);
            assert!((top(&shifted, 1) - 2.0_f32.mul_add(height, top(&plain, 1))).abs() < 0.01);
            assert!((shifted.rect.height() - 2.0_f32.mul_add(height, plain.rect.height())).abs() < 0.01);

            // `b` of line 1 is char 3: the cursor is drawn at the moved row, and a click there finds it.
            let rect = shifted.pos_from_cursor(CCursor::new(3));
            assert!((rect.top() - top(&shifted, 1)).abs() < 0.01);
            assert_eq!(shifted.cursor_from_pos(rect.center().to_vec2()).index.0, 3);
            // A click in the gap goes to a neighbor line, not a wrong place far away.
            let in_gap = egui::vec2(0.0, top(&shifted, 1) - height);
            assert!(shifted.cursor_from_pos(in_gap).index.0 <= 3);
        });
    }
}
