//! Text changes: operators (`d`, `c`, `y`, `>`, `<`, `gu`, `gU`, `g~`), paste, join,
//! `Ctrl+A` / `Ctrl+X`, and the Visual-mode versions of them.

use std::ops::Range;

use super::{
    Mode, VimState, buffer::Buffer, command::Operator, exec::Exec, keys::VimKey,
    registers::Register,
};

impl VimState {
    /// Applies `op` to `range`. For linewise ranges, `range` only marks the first/last line;
    /// whole lines are used.
    pub(super) fn operate(
        &mut self,
        op: Operator,
        range: Range<usize>,
        linewise: bool,
        exec: &mut Exec,
    ) {
        let buf = &exec.buf;
        let (start, end) = if linewise {
            (buf.line_start(range.start), buf.line_end(range.end))
        } else {
            (range.start, range.end.min(buf.len()))
        };
        if start > end || (start == end && !linewise) {
            return;
        }

        match op {
            Operator::Yank => {
                self.yank(buf.slice(start..end), linewise, true, exec);
                self.cursor = start;
            }
            Operator::Delete => {
                self.yank(buf.slice(start..end), linewise, false, exec);
                let delete =
                    if linewise { with_line_break(&exec.buf, start, end) } else { start..end };
                exec.edit(self.cursor, delete.clone(), "");
                let at = delete.start.min(exec.buf.len());
                self.cursor =
                    if linewise { exec.buf.first_non_blank(at) } else { exec.buf.clamp_normal(at) };
            }
            Operator::Change => {
                self.yank(buf.slice(start..end), linewise, false, exec);
                // `cc` keeps the (now empty) line to type into.
                exec.edit(self.cursor, start..end, "");
                self.cursor = start;
                self.mode = Mode::Insert;
            }
            Operator::Indent | Operator::Outdent => {
                let (first, last) = (buf.line_of(start), buf.line_of(end));
                self.shift_lines(first, last, op == Operator::Indent, exec);
                self.cursor = exec.buf.first_non_blank(exec.buf.nth_line_start(first));
            }
            Operator::Lower | Operator::Upper | Operator::ToggleCase => {
                let changed: String =
                    buf.chars[start..end].iter().flat_map(|&c| change_case(c, op)).collect();
                exec.edit(self.cursor, start..end, &changed);
                self.cursor = start;
            }
        }
        if op != Operator::Change && self.mode.is_visual() {
            self.mode = Mode::Normal;
        }
    }

    fn yank(&mut self, text: String, linewise: bool, is_yank: bool, exec: &Exec) {
        let register = Register { text, linewise, block: false };
        self.registers.store(self.active_register, register, is_yank, &exec.ctx);
    }

    /// `>` (`right`) / `<` on lines `first..=last`, by `shift_width` spaces.
    pub(super) fn shift_lines(&self, first: usize, last: usize, right: bool, exec: &mut Exec) {
        let width = self.options.shift_width.max(1);
        let start = exec.buf.nth_line_start(first);
        let end = exec.buf.line_end(exec.buf.nth_line_start(last));
        let block = exec.buf.slice(start..end);
        let shifted: Vec<String> = block
            .split('\n')
            .map(|line| {
                if right {
                    if line.is_empty() {
                        String::new()
                    } else {
                        format!("{}{line}", " ".repeat(width))
                    }
                } else if let Some(rest) = line.strip_prefix('\t') {
                    rest.to_string()
                } else {
                    let spaces = line.chars().take(width).take_while(|c| *c == ' ').count();
                    line[spaces..].to_string()
                }
            })
            .collect();
        exec.edit(start, start..end, &shifted.join("\n"));
    }

    /// `p` / `P`. Linewise text goes on its own line; charwise text after/before the cursor.
    pub(super) fn paste(&mut self, after: bool, count: usize, exec: &mut Exec) {
        let register = self.registers.get(self.active_register).clone();
        self.paste_register(&register, after, count, exec);
    }

    fn paste_register(&mut self, register: &Register, after: bool, count: usize, exec: &mut Exec) {
        if register.text.is_empty() && !register.linewise {
            return;
        }
        if register.block {
            for _ in 0..count {
                self.paste_block(&register.text, after, exec);
            }
            return;
        }
        let cur = self.cursor;

        if register.linewise {
            let lines = vec![register.text.as_str(); count].join("\n");
            if after {
                let at = exec.buf.line_end(cur);
                exec.edit(cur, at..at, &format!("\n{lines}"));
                self.cursor = exec.buf.first_non_blank(at + 1);
            } else {
                let at = exec.buf.line_start(cur);
                exec.edit(cur, at..at, &format!("{lines}\n"));
                self.cursor = exec.buf.first_non_blank(at);
            }
        } else {
            let text = register.text.repeat(count);
            let line_empty = exec.buf.line_start(cur) == exec.buf.line_end(cur);
            let at = if after && !line_empty { cur + 1 } else { cur };
            exec.edit(cur, at..at, &text);
            self.cursor = at + text.chars().count().saturating_sub(1);
        }
    }

    /// `J` (`spaces`) / `gJ`: joins `count` lines (at least 2).
    pub(super) fn join_lines(&mut self, count: usize, spaces: bool, exec: &mut Exec) {
        exec.snapshot(self.cursor);
        for _ in 0..count.max(2) - 1 {
            let newline = exec.buf.line_end(self.cursor);
            if newline >= exec.buf.len() {
                break;
            }
            if !spaces {
                exec.buf.replace(newline..newline + 1, "");
                self.cursor = newline;
                continue;
            }
            // Remove `\n` + the next line's indentation, then put one space (unless a side is empty).
            let mut end = newline + 1;
            while exec.buf.chars.get(end).is_some_and(|c| *c == ' ' || *c == '\t') {
                end += 1;
            }
            let left_empty = newline == exec.buf.line_start(newline);
            let right_empty = exec.buf.chars.get(end).is_none_or(|c| *c == '\n');
            let sep = if left_empty || right_empty { "" } else { " " };
            exec.buf.replace(newline..end, sep);
            self.cursor = newline;
        }
        exec.output.changed = true;
    }

    /// `Ctrl+A` (`up`) / `Ctrl+X`: adds `count` to the number at or after the cursor on the line.
    ///
    /// e.g., `0.100000 x` with the cursor on `0` -> `1.100000 x`; `-1` -> `0`
    pub(super) fn increment(&mut self, count: usize, up: bool, exec: &mut Exec) {
        let buf = &exec.buf;
        let end_of_line = buf.line_end(self.cursor);
        // The number under the cursor, else the next one on the line.
        let Some(digit) = (self.cursor..end_of_line).find(|&j| buf.chars[j].is_ascii_digit())
        else {
            return;
        };
        let mut start = digit;
        while start > 0 && buf.chars[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start > 0 && buf.chars[start - 1] == '-' {
            start -= 1;
        }
        let mut end = digit;
        while end < end_of_line && buf.chars[end].is_ascii_digit() {
            end += 1;
        }

        let Ok(value) = buf.slice(start..end).parse::<i64>() else { return };
        let delta = i64::try_from(count).unwrap_or(i64::MAX);
        let value = if up { value.saturating_add(delta) } else { value.saturating_sub(delta) };
        let text = value.to_string();
        exec.edit(self.cursor, start..end, &text);
        self.cursor = start + text.chars().count() - 1;
    }

    // ── Visual mode ──────────────────────────────────────────────────────────

    /// The selection as a range (charwise: up to and including the cursor char) and whether
    /// it is linewise.
    fn visual_range(&self, buf: &Buffer) -> (Range<usize>, bool) {
        let (lo, hi) = (self.anchor.min(self.cursor), self.anchor.max(self.cursor));
        if self.mode == Mode::VisualLine {
            (lo..hi, true)
        } else {
            (lo..(hi + 1).min(buf.len()), false)
        }
    }

    /// Visual-mode keys that act on the selection right away. Returns `false` if `key` is not
    /// one of them (it is then parsed as a normal command, e.g. a motion).
    ///
    /// `d x y c s S > < ~ u U J p P o O I A` and `gu gU g~` (after a pending `g`)
    pub(super) fn visual_operator(&mut self, key: VimKey, exec: &mut Exec) -> bool {
        if self.mode == Mode::VisualBlock {
            return self.block_operator(key, exec);
        }
        let VimKey::Char(c) = key else { return false };
        let after_g = self.pending == [VimKey::Char('g')];
        if !self.pending.is_empty() && !after_g {
            return false;
        }
        let op = match (after_g, c) {
            (false, 'd' | 'x') => Operator::Delete,
            (false, 'y') => Operator::Yank,
            (false, 'c' | 's') => Operator::Change,
            (false, '>') => Operator::Indent,
            (false, '<') => Operator::Outdent,
            (_, '~') => Operator::ToggleCase,
            (_, 'u') => Operator::Lower,
            (_, 'U') => Operator::Upper,
            (false, 'S' | 'J' | 'p' | 'P' | 'o' | 'O' | 'I' | 'A') => {
                return self.visual_special(c, exec);
            }
            _ => return false,
        };
        self.pending.clear();

        let (range, linewise) = self.visual_range(&exec.buf);
        let linewise = linewise || matches!(op, Operator::Indent | Operator::Outdent);
        self.cursor = range.start;
        self.operate(op, range, linewise, exec);
        self.mode = if self.mode == Mode::Insert { Mode::Insert } else { Mode::Normal };
        true
    }

    /// Visual keys that are not operators.
    fn visual_special(&mut self, c: char, exec: &mut Exec) -> bool {
        let (range, linewise) = self.visual_range(&exec.buf);
        let (lo, hi) = (range.start, range.end.saturating_sub(1).max(range.start));
        match c {
            // `S`: change whole lines.
            'S' => {
                self.cursor = lo;
                self.operate(Operator::Change, lo..hi, true, exec);
            }
            'J' => {
                let lines = exec.buf.line_of(hi) - exec.buf.line_of(lo) + 1;
                self.cursor = lo;
                self.mode = Mode::Normal;
                self.join_lines(lines, true, exec);
            }
            // Replace the selection with the register (the old text goes to the register).
            'p' | 'P' => {
                let register = self.registers.get(self.active_register).clone();
                self.cursor = lo;
                self.operate(Operator::Delete, range, linewise, exec);
                // Deleted up to the line end: the cursor stepped back onto the char before.
                let at_line_end = !register.linewise
                    && lo >= exec.buf.line_end(lo)
                    && lo > exec.buf.line_start(lo);
                self.paste_register(&register, at_line_end, 1, exec);
            }
            // The other end of the selection.
            'o' | 'O' => std::mem::swap(&mut self.anchor, &mut self.cursor),
            'I' => {
                self.cursor = if linewise { exec.buf.line_start(lo) } else { lo };
                self.mode = Mode::Insert;
            }
            'A' => {
                self.cursor = if linewise { exec.buf.line_end(hi) } else { range.end };
                self.mode = Mode::Insert;
            }
            _ => return false,
        }
        true
    }
}

/// Lines `start..end` plus one line break, so deleting them removes the lines entirely.
/// On the last line the break *before* it is taken instead.
const fn with_line_break(buf: &Buffer, start: usize, end: usize) -> Range<usize> {
    if end < buf.len() { start..end + 1 } else { start.saturating_sub(1)..end }
}

/// `c` with `gu` (lower), `gU` (upper) or `g~` (toggle).
pub(super) fn change_case(c: char, op: Operator) -> Vec<char> {
    let lower = || c.to_lowercase().collect();
    let upper = || c.to_uppercase().collect();
    match op {
        Operator::Lower => lower(),
        Operator::Upper => upper(),
        _ if c.is_uppercase() => lower(),
        _ => upper(),
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

    #[test]
    fn increments_numbers() {
        let mut vim = VimState::default();
        let mut e = exec("time 9 x");
        vim.increment(1, true, &mut e);
        assert_eq!(e.buf.text(), "time 10 x");
        assert_eq!(vim.cursor, 6);

        let mut vim = VimState::default();
        let mut e = exec("-1");
        vim.increment(3, true, &mut e);
        assert_eq!(e.buf.text(), "2");
    }
}
