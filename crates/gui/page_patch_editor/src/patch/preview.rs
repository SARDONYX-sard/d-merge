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
    /// The editor line the preview follows, and its row (highlighted so you see where you are).
    followed: Option<(usize, Option<usize>)>,
    /// A pending scroll to the followed row.
    scroll_to_followed: bool,
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
        Self {
            lines,
            longest_line,
            blocks,
            current: 0,
            scroll_to_current: false,
            followed: None,
            scroll_to_followed: false,
        }
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

    /// The editor's cursor is on `line`: highlights the row showing it and scrolls there.
    /// Does nothing while the line stays the same, so you can scroll the preview freely.
    pub(super) fn follow_editor_line(&mut self, line: usize, editor_text: &str) {
        if self.followed.is_some_and(|(followed, _)| followed == line) {
            return;
        }
        let row = self.row_of_editor_line(line, editor_text);
        self.followed = Some((line, row));
        self.scroll_to_followed = row.is_some();
    }

    /// The highlighted row. See [`Self::follow_editor_line`].
    pub(super) fn followed_row(&self) -> Option<usize> {
        self.followed.and_then(|(_, row)| row)
    }

    /// The pending scroll: the followed row, which goes to the middle of the view.
    pub(super) fn take_followed_scroll(&mut self) -> Option<usize> {
        std::mem::take(&mut self.scroll_to_followed).then(|| self.followed_row()).flatten()
    }

    /// The preview row showing editor line `line`. The reverse of [`Self::editor_line_of`]: the n-th
    /// line with this text in the editor is the n-th one in the preview.
    fn row_of_editor_line(&self, line: usize, editor_text: &str) -> Option<usize> {
        let target = editor_text.lines().nth(line)?.trim();
        let occurrence = editor_text.lines().take(line).filter(|other| other.trim() == target).count();
        let is_mod_line = |kind: Kind| matches!(kind, Kind::Common | Kind::New);
        let rows = self.lines.iter().enumerate();
        let mut same = rows.filter(|(_, (kind, text))| is_mod_line(*kind) && text.trim() == target);
        same.nth(occurrence).map(|(row, _)| row)
    }

    /// The line of the editor text that preview row `row` shows, to jump there.
    ///
    /// The preview is normalized, so its line numbers can differ from the editor's: the n-th line
    /// with this text in the preview is looked up as the n-th one in the editor. A row that is not
    /// in the editor text (the template's lines, the markers) goes to the next line that is.
    ///
    /// e.g., the row of the 2nd `<hkparam name="enable">false</hkparam>` -> that line's number.
    pub(super) fn editor_line_of(&self, row: usize, editor_text: &str) -> Option<usize> {
        let is_mod_line = |kind: Kind| matches!(kind, Kind::Common | Kind::New);
        let next = self.lines.get(row..)?.iter().position(|(kind, _)| is_mod_line(*kind)).map(|at| row + at);
        let target_row = next.or_else(|| self.lines.iter().rposition(|(kind, _)| is_mod_line(*kind)))?;
        let target = self.lines[target_row].1.trim();

        let mod_lines_before = || self.lines[..target_row].iter().filter(|(kind, _)| is_mod_line(*kind));
        let occurrence = mod_lines_before().filter(|(_, line)| line.trim() == target).count();
        let found = editor_text.lines().enumerate().filter(|(_, line)| line.trim() == target).nth(occurrence);
        // Not found (e.g., pointers joined on a line there): about the same place.
        let last_line = editor_text.lines().count().saturating_sub(1);
        Some(found.map_or_else(|| mod_lines_before().count().min(last_line), |(line, _)| line))
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
    fn follows_the_editor_line() {
        let patch = "a\nx\n<!-- MOD_CODE ~m~ OPEN -->\nnew\nx\n<!-- ORIGINAL -->\nold\n<!-- CLOSE -->\nb\n";
        let mut preview = Preview::new(patch);
        let editor = "a\nx\nnew\n\t\tx\nb\n";

        preview.follow_editor_line(3, editor); // the 2nd `x`
        assert_eq!(preview.followed_row(), Some(4));
        assert_eq!(preview.take_followed_scroll(), Some(4));
        assert_eq!(preview.take_followed_scroll(), None);

        preview.follow_editor_line(3, editor); // same line: the view stays where you scrolled it
        assert_eq!(preview.take_followed_scroll(), None);

        preview.follow_editor_line(99, editor); // no such line
        assert_eq!(preview.followed_row(), None);
    }

    #[test]
    fn rows_map_to_editor_lines() {
        let patch = "a\nx\n<!-- MOD_CODE ~m~ OPEN -->\nnew\nx\n<!-- ORIGINAL -->\nold\n<!-- CLOSE -->\nb\n";
        let preview = Preview::new(patch);
        let editor = "a\nx\nnew\n\t\tx\nb\n"; // indented differently, on purpose
        let line = |row| preview.editor_line_of(row, editor);

        assert_eq!(line(0), Some(0)); // a
        assert_eq!(line(1), Some(1)); // x
        assert_eq!(line(2), Some(2)); // OPEN: the next line of the mod, `new`
        assert_eq!(line(4), Some(3)); // the 2nd `x`
        assert_eq!(line(6), Some(4)); // the template's `old`: not in the editor, goes on to `b`
        assert_eq!(line(99), None);
    }

    #[test]
    fn no_blocks_nothing_to_step() {
        let mut preview = Preview::new("a\n");
        preview.step(true);
        assert_eq!(preview.current, 0);
        assert_eq!(preview.take_scroll_row(), None);
    }
}
