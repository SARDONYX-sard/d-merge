//! Visual block mode (`Ctrl+V`): a rectangle of columns over several lines.
//!
//! ```text
//! d x  delete the block      y  yank it (pasted as a block by p / P)
//! c s  change: type, Esc applies the text to every line
//! I A  insert before / append after the block on every line
//! r<c> replace every char      ~ u U  change case     > <  shift the lines
//! o    other corner            O  other side on the same line
//! ```
//! Columns are char columns. egui has a single text selection, so the editor draws the block
//! itself from [`VimState::block_ranges`].

use std::ops::Range;

use super::{
    Mode, VimState, buffer::Buffer, command::Operator, exec::Exec, keys::VimKey,
    registers::Register,
};

/// Text typed in Insert mode after `I` / `A` / `c` on a block, to be copied to the other lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BlockInsert {
    /// The other lines (the first one gets the typed text directly).
    lines: (usize, usize),
    column: usize,
    /// `A`: short lines are padded with spaces up to `column` (`I` skips them).
    append: bool,
}

impl VimState {
    /// The block: (first line, last line, first column, last column), all inclusive.
    fn block(&self, buf: &Buffer) -> (usize, usize, usize, usize) {
        let pos = |i: usize| (buf.line_of(i), i - buf.line_start(i));
        let ((l1, c1), (l2, c2)) = (pos(self.anchor), pos(self.cursor));
        (l1.min(l2), l1.max(l2), c1.min(c2), c1.max(c2))
    }

    /// Char ranges of the block, one per line (empty for lines shorter than the block).
    pub(super) fn block_ranges_in(&self, buf: &Buffer) -> Vec<Range<usize>> {
        let (first, last, left, right) = self.block(buf);
        (first..=last)
            .map(|line| {
                let start = buf.nth_line_start(line);
                let len = buf.line_end(start) - start;
                (start + left.min(len))..(start + (right + 1).min(len))
            })
            .collect()
    }

    /// The block to draw, if Visual block mode is on.
    pub(crate) fn block_ranges(&self, text: &str) -> Option<Vec<Range<usize>>> {
        (self.mode == Mode::VisualBlock).then(|| self.block_ranges_in(&Buffer::new(text)))
    }

    /// Block-mode keys. Returns `false` if `key` is not one of them (e.g., a motion).
    pub(super) fn block_operator(&mut self, key: VimKey, exec: &mut Exec) -> bool {
        let VimKey::Char(c) = key else { return false };
        // `r<c>`: the char comes as the next key.
        if self.pending == [VimKey::Char('r')] {
            self.pending.clear();
            self.block_replace(c, exec);
            return true;
        }
        if !self.pending.is_empty() {
            return false;
        }

        let (first, last, left, right) = self.block(&exec.buf);
        match c {
            'r' => {
                self.pending.push(key);
                return true;
            }
            'd' | 'x' => self.block_delete(true, exec),
            'y' => {
                self.block_yank(exec);
                self.leave_block(left, first, exec);
            }
            'c' | 's' => {
                self.block_delete(true, exec);
                self.start_block_insert(first, last, left, false, exec);
            }
            'I' => self.start_block_insert(first, last, left, false, exec),
            'A' => self.start_block_insert(first, last, right + 1, true, exec),
            '~' | 'u' | 'U' => {
                let op = match c {
                    'u' => Operator::Lower,
                    'U' => Operator::Upper,
                    _ => Operator::ToggleCase,
                };
                self.block_case(op, exec);
                self.leave_block(left, first, exec);
            }
            '>' | '<' => {
                self.shift_lines(first, last, c == '>', exec);
                self.mode = Mode::Normal;
                self.cursor = exec.buf.first_non_blank(exec.buf.nth_line_start(first));
            }
            'o' => std::mem::swap(&mut self.anchor, &mut self.cursor),
            'O' => self.swap_block_sides(&exec.buf),
            _ => return false,
        }
        true
    }

    fn leave_block(&mut self, column: usize, line: usize, exec: &Exec) {
        self.mode = Mode::Normal;
        let start = exec.buf.nth_line_start(line);
        self.cursor = exec.buf.clamp_normal(start + column.min(exec.buf.line_end(start) - start));
    }

    /// The block as text: its lines joined by `\n`.
    fn block_text(&self, buf: &Buffer) -> String {
        self.block_ranges_in(buf)
            .into_iter()
            .map(|range| buf.slice(range))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn block_yank(&mut self, exec: &Exec) {
        let register = Register { text: self.block_text(&exec.buf), linewise: false, block: true };
        self.registers.store(self.active_register, register, true, &exec.ctx);
    }

    /// `d`: removes the block from every line (bottom-up, so earlier indices stay valid).
    fn block_delete(&mut self, yank: bool, exec: &mut Exec) {
        let (first, _, left, _) = self.block(&exec.buf);
        if yank {
            let register =
                Register { text: self.block_text(&exec.buf), linewise: false, block: true };
            self.registers.store(self.active_register, register, false, &exec.ctx);
        }
        exec.snapshot(self.cursor);
        for range in self.block_ranges_in(&exec.buf).into_iter().rev() {
            exec.buf.replace(range, "");
        }
        exec.output.changed = true;
        self.leave_block(left, first, exec);
    }

    fn block_replace(&mut self, with: char, exec: &mut Exec) {
        let (first, _, left, _) = self.block(&exec.buf);
        exec.snapshot(self.cursor);
        for range in self.block_ranges_in(&exec.buf).into_iter().rev() {
            let replaced = with.to_string().repeat(range.len());
            exec.buf.replace(range, &replaced);
        }
        exec.output.changed = true;
        self.leave_block(left, first, exec);
    }

    fn block_case(&self, op: Operator, exec: &mut Exec) {
        exec.snapshot(self.cursor);
        for range in self.block_ranges_in(&exec.buf).into_iter().rev() {
            let changed: String = exec.buf.chars[range.clone()]
                .iter()
                .flat_map(|&c| super::edit::change_case(c, op))
                .collect();
            exec.buf.replace(range, &changed);
        }
        exec.output.changed = true;
    }

    /// `O`: the cursor goes to the other side of the block, on its own line.
    fn swap_block_sides(&mut self, buf: &Buffer) {
        let column = |i: usize| i - buf.line_start(i);
        let (anchor_col, cursor_col) = (column(self.anchor), column(self.cursor));
        let line_start = |i: usize| buf.line_start(i);
        let clamp = |start: usize, col: usize| start + col.min(buf.line_end(start) - start);
        let (anchor_start, cursor_start) = (line_start(self.anchor), line_start(self.cursor));
        self.anchor = clamp(anchor_start, cursor_col);
        self.cursor = clamp(cursor_start, anchor_col);
    }

    /// `I` / `A` / `c`: Insert mode on the first line; the typed text is copied to the other
    /// lines when Insert mode is left ([`Self::finish_block_insert`]).
    fn start_block_insert(
        &mut self,
        first: usize,
        last: usize,
        column: usize,
        append: bool,
        exec: &mut Exec,
    ) {
        let start = exec.buf.nth_line_start(first);
        let len = exec.buf.line_end(start) - start;
        if append && len < column {
            exec.edit(self.cursor, start + len..start + len, &" ".repeat(column - len));
        }
        self.cursor = start + column.min(exec.buf.line_end(start) - start);
        self.mode = Mode::Insert;
        self.block_insert =
            (last > first).then_some(BlockInsert { lines: (first + 1, last), column, append });
    }

    /// Copies `typed` (the text typed on the first line) to the other lines of the block.
    pub(super) fn finish_block_insert(&mut self, typed: Option<&str>, exec: &mut Exec) {
        let Some(BlockInsert { lines: (first, last), column, append }) = self.block_insert.take()
        else {
            return;
        };
        // Only plain one-line insertions are repeated, like Vim.
        let Some(text) = typed.filter(|t| !t.is_empty() && !t.contains('\n')) else { return };

        exec.snapshot(self.cursor);
        for line in (first..=last).rev() {
            let start = exec.buf.nth_line_start(line);
            let len = exec.buf.line_end(start) - start;
            if len < column {
                if !append {
                    continue; // `I` skips lines that end before the block
                }
                exec.buf.replace(start + len..start + len, &" ".repeat(column - len));
            }
            exec.buf.replace(start + column..start + column, text);
        }
        exec.output.changed = true;
    }

    /// `p` / `P` of a block register: each of its lines goes to the next line, at the cursor
    /// column (lines are added / padded as needed).
    pub(super) fn paste_block(&mut self, text: &str, after: bool, exec: &mut Exec) {
        let cur = self.cursor;
        let first_line = exec.buf.line_of(cur);
        let line_start = exec.buf.line_start(cur);
        let empty_line = line_start == exec.buf.line_end(cur);
        let column = cur - line_start + usize::from(after && !empty_line);

        exec.snapshot(cur);
        for (n, part) in text.split('\n').enumerate() {
            let line = first_line + n;
            if line >= exec.buf.line_count() {
                let end = exec.buf.len();
                exec.buf.replace(end..end, "\n");
            }
            let start = exec.buf.nth_line_start(line);
            let len = exec.buf.line_end(start) - start;
            if len < column {
                exec.buf.replace(start + len..start + len, &" ".repeat(column - len));
            }
            exec.buf.replace(start + column..start + column, part);
        }
        exec.output.changed = true;
        self.cursor = line_start + column;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{KeymapOutput, vim::history::History};

    fn exec(text: &str) -> Exec {
        Exec {
            buf: Buffer::new(text),
            history: History::default(),
            ctx: egui::Context::default(),
            output: KeymapOutput::default(),
            edits: 0,
        }
    }

    fn block(anchor: usize, cursor: usize) -> VimState {
        VimState { mode: Mode::VisualBlock, anchor, cursor, ..VimState::default() }
    }

    #[test]
    fn delete_yank_and_paste_block() {
        let mut e = exec("abcd\nefgh\nij");
        let mut vim = block(1, 7); // `b`..`g` (columns 1..=2, lines 0..=1)
        assert_eq!(vim.block_ranges_in(&e.buf), vec![1..3, 6..8]);
        vim.block_operator(VimKey::Char('d'), &mut e);
        assert_eq!(e.buf.text(), "ad\neh\nij");
        assert_eq!(vim.mode, Mode::Normal);

        vim.cursor = 0;
        vim.paste_block("bc\nfg", true, &mut e);
        assert_eq!(e.buf.text(), "abcd\nefgh\nij");
    }

    #[test]
    fn insert_on_every_line() {
        let mut e = exec("ab\ncd\n\nef");
        let mut vim = block(0, 9); // column 0, lines 0..=3
        vim.block_operator(VimKey::Char('I'), &mut e);
        assert_eq!(vim.mode, Mode::Insert);
        e.buf.replace(0..0, "- "); // typed on the first line
        vim.finish_block_insert(Some("- "), &mut e);
        assert_eq!(e.buf.text(), "- ab\n- cd\n- \n- ef");
    }
}
