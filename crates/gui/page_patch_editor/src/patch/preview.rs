//! The patch preview's data: the `#XXXX.txt` of the open object, split into lines to draw like a
//! git diff (see [`super::ui::preview`]).
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
pub(super) enum Kind {
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
    /// Chars of the longest line: the width of the horizontal scroll.
    longest_line: usize,
    blocks: usize,
    /// The block ▲ / ▼ moved to (0-based).
    pub(super) current: usize,
    /// A pending scroll to `current`.
    scroll_to_current: bool,
}

impl Preview {
    pub(super) fn new(patch: &str) -> Self {
        let mut lines = Vec::new();
        let mut kind = Kind::Common;
        let mut blocks = 0;
        let mut longest_line = 0;
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
            let line = line.replace('\t', "    ");
            longest_line = longest_line.max(line.chars().count());
            lines.push((line_kind, line));
        }
        Self { lines, longest_line, blocks, current: 0, scroll_to_current: false }
    }

    pub(super) fn lines(&self) -> &[(Kind, String)] {
        &self.lines
    }

    pub(super) const fn longest_line(&self) -> usize {
        self.longest_line
    }

    pub(super) const fn blocks(&self) -> usize {
        self.blocks
    }

    /// Keeps the ▲ / ▼ position (clamped) after the patch was recomputed.
    pub(super) fn keep_current(&mut self, current: usize) {
        self.current = current.min(self.blocks.saturating_sub(1));
    }

    /// ▲ / ▼: the previous / next block, wrapping around, and a scroll to it.
    pub(super) const fn step(&mut self, forward: bool) {
        if self.blocks == 0 {
            return;
        }
        self.current =
            if forward { (self.current + 1) % self.blocks } else { (self.current + self.blocks - 1) % self.blocks };
        self.scroll_to_current = true;
    }

    /// The pending scroll: the row of the current block's `OPEN` line, which goes to the top.
    pub(super) fn take_scroll_row(&mut self) -> Option<usize> {
        if !std::mem::take(&mut self.scroll_to_current) {
            return None;
        }
        self.lines.iter().position(|(kind, _)| *kind == Kind::Open(self.current))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "a\n<!-- MOD_CODE ~m~ OPEN -->\nb\n<!-- ORIGINAL -->\nc\n<!-- CLOSE -->\nd\n\
                         <!-- MOD_CODE ~m~ OPEN -->\n\t\tlong line\n<!-- CLOSE -->\n";

    #[test]
    fn splits_blocks() {
        let preview = Preview::new(PATCH);
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
                Kind::Common,
                Kind::Open(1),
                Kind::New,
                Kind::Marker
            ]
        );
        assert_eq!(preview.blocks, 2);
        assert_eq!(preview.longest_line, "<!-- MOD_CODE ~m~ OPEN -->".len());
    }

    #[test]
    fn steps_wrap_and_scroll_to_the_open_line() {
        let mut preview = Preview::new(PATCH);
        assert_eq!(preview.take_scroll_row(), None);
        preview.step(true);
        assert_eq!(preview.current, 1);
        assert_eq!(preview.take_scroll_row(), Some(7));
        assert_eq!(preview.take_scroll_row(), None); // only once
        preview.step(true);
        assert_eq!(preview.current, 0);
        preview.step(false);
        assert_eq!(preview.current, 1);
    }

    #[test]
    fn no_blocks_nothing_to_step() {
        let mut preview = Preview::new("a\n");
        preview.step(true);
        assert_eq!(preview.current, 0);
        assert_eq!(preview.take_scroll_row(), None);
    }
}
