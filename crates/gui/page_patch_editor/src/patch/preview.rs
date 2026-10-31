//! The patch preview: the `#XXXX.txt` of the open object, drawn like a git diff.
//!
//! ```text
//! Patch preview  ▲ ▼  2/5  ✔
//!   <hkparam name="userData">1</hkparam>
//! ↶ <!-- MOD_CODE ~mymod~ OPEN -->           <- `↶`: put this block back to the template
//!   <hkparam name="enable">false</hkparam>   (green: the mod's lines)
//!   <!-- ORIGINAL -->
//!   <hkparam name="enable">true</hkparam>    (red: the template's lines)
//!   <!-- CLOSE -->
//! ```

/// What a preview line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Common,
    /// `<!-- MOD_CODE ~..~ OPEN -->` of block N.
    Open(usize),
    New,
    Marker,
    Old,
}

/// A patch split into lines for drawing.
#[derive(Debug, Default)]
pub(super) struct Preview {
    lines: Vec<(Kind, String)>,
    blocks: usize,
    /// The block ▲ / ▼ moved to (0-based), and a pending scroll to it.
    pub(super) current: usize,
    scroll_to_current: bool,
}

impl Preview {
    pub(super) fn new(patch: &str) -> Self {
        let mut lines = Vec::new();
        let mut kind = Kind::Common;
        let mut blocks = 0;
        for line in patch.lines() {
            let trimmed = line.trim();
            let line_kind = if trimmed.starts_with("<!-- MOD_CODE ~") {
                kind = Kind::New;
                blocks += 1;
                Kind::Open(blocks - 1)
            } else if trimmed == "<!-- ORIGINAL -->" {
                kind = Kind::Old;
                Kind::Marker
            } else if trimmed == "<!-- CLOSE -->" {
                kind = Kind::Common;
                Kind::Marker
            } else {
                kind
            };
            lines.push((line_kind, line.replace('\t', "    ")));
        }
        Self { lines, blocks, current: 0, scroll_to_current: false }
    }

    /// Keeps the ▲ / ▼ position (clamped) after the patch was recomputed.
    pub(super) fn keep_current(&mut self, current: usize) {
        self.current = current.min(self.blocks.saturating_sub(1));
    }

    /// `▲ ▼ 2/5`: moves the current block (and scrolls to it).
    pub(super) fn nav_ui(&mut self, ui: &mut egui::Ui, hover: &str) {
        if self.blocks == 0 {
            return;
        }
        for (forward, glyph) in [(false, "▲"), (true, "▼")] {
            if ui.small_button(glyph).on_hover_text(hover).clicked() {
                self.current = if forward {
                    (self.current + 1) % self.blocks
                } else {
                    (self.current + self.blocks - 1) % self.blocks
                };
                self.scroll_to_current = true;
            }
        }
        ui.label(format!("{}/{}", self.current + 1, self.blocks));
    }

    /// The lines (only the visible ones are laid out). Returns the block whose `↶` was clicked.
    pub(super) fn ui(&mut self, ui: &mut egui::Ui, revert_hover: &str) -> Option<usize> {
        let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 2.0;
        let green = egui::Color32::from_rgb(46, 160, 67).gamma_multiply(0.18);
        let red = egui::Color32::from_rgb(248, 81, 73).gamma_multiply(0.18);
        let current_tint = ui.visuals().selection.bg_fill.gamma_multiply(0.35);

        let mut scroll = egui::ScrollArea::both().auto_shrink(false);
        if std::mem::take(&mut self.scroll_to_current)
            && let Some(row) = self.lines.iter().position(|(k, _)| *k == Kind::Open(self.current))
        {
            #[expect(clippy::cast_precision_loss, reason = "row counts are small")]
            let offset = (row as f32 - 2.0).max(0.0) * row_height;
            scroll = scroll.vertical_scroll_offset(offset);
        }

        let mut revert = None;
        let current = self.current;
        scroll.show_rows(ui, row_height, self.lines.len(), |ui, range| {
            for (kind, text) in &self.lines[range] {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().max(600.0), row_height),
                    egui::Sense::hover(),
                );
                let fill = match kind {
                    Kind::New => Some(green),
                    Kind::Old => Some(red),
                    Kind::Open(i) if *i == current => Some(current_tint),
                    _ => None,
                };
                if let Some(fill) = fill {
                    ui.painter().rect_filled(rect, 0.0, fill);
                }
                // `↶` at the start of each block, like git's conflict buttons.
                let gutter = egui::Rect::from_min_size(rect.min, egui::vec2(20.0, row_height));
                if let Kind::Open(i) = kind
                    && ui
                        .put(gutter, egui::Button::new("↶").small())
                        .on_hover_text(revert_hover)
                        .clicked()
                {
                    revert = Some(*i);
                }
                let color = match kind {
                    Kind::Open(_) | Kind::Marker => ui.visuals().weak_text_color(),
                    _ => ui.visuals().text_color(),
                };
                ui.painter().text(
                    rect.left_center() + egui::vec2(24.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::TextStyle::Monospace.resolve(ui.style()),
                    color,
                );
            }
        });
        revert
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_blocks() {
        let patch = "a\n<!-- MOD_CODE ~m~ OPEN -->\nb\n<!-- ORIGINAL -->\nc\n<!-- CLOSE -->\nd\n";
        let preview = Preview::new(patch);
        let kinds: Vec<_> = preview.lines.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            [
                Kind::Common,
                Kind::Open(0),
                Kind::New,
                Kind::Marker,
                Kind::Old,
                Kind::Marker,
                Kind::Common
            ]
        );
        assert_eq!(preview.blocks, 1);
    }
}
