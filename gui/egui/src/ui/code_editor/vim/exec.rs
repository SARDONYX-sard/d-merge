//! Executes parsed Vim commands on a [`Buffer`].
//!
//! # Ranges
//! Operators (`d`, `c`, `y`) act on a range computed from a motion. Like Vim, a motion is one of:
//! - **exclusive**: the target char is not included (e.g., `dw` stops before the next word)
//! - **inclusive**: the target char is included (e.g., `de`, `d$`)
//! - **linewise**: whole lines (e.g., `dj`, `dd`, `dG`)

use std::ops::Range;

use egui::text::{CCursor, CCursorRange};

use super::{
    Mode, Register, VimOutput, VimState,
    buffer::Buffer,
    command::{Command, Motion, Operator, Simple, Target},
    keys::VimKey,
};

/// Same type as egui's (non-exported) `TextEditUndoer`: what `TextEditState::undoer()` returns.
pub(super) type TextEditUndoer = egui::util::undoer::Undoer<(CCursorRange, String)>;

/// Mutable context while running a batch of keys.
pub(super) struct Exec {
    pub(super) buf: Buffer,
    /// The `TextEdit`'s undo history; Vim edits are recorded into it.
    pub(super) undoer: TextEditUndoer,
    /// For copying yanks to the clipboard.
    pub(super) ctx: egui::Context,
    pub(super) output: VimOutput,
}

impl Exec {
    /// Records the state before an edit, so `u` can restore it.
    fn snapshot(&mut self, cursor: usize) {
        let state = (CCursorRange::one(CCursor::new(cursor)), self.buf.text());
        self.undoer.add_undo(&state);
    }

    fn edit(&mut self, cursor: usize, range: Range<usize>, with: &str) {
        self.snapshot(cursor);
        self.buf.replace(range, with);
        self.output.changed = true;
    }
}

/// How a motion's range is interpreted by operators.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Exclusive,
    Inclusive,
    Linewise,
}

impl VimState {
    pub(super) fn execute(
        &mut self,
        command: Command,
        count: usize,
        explicit_count: bool,
        exec: &mut Exec,
    ) {
        match command {
            Command::Move(motion) => {
                let (target, _) =
                    self.motion_target(motion, count, explicit_count, &exec.buf, false);
                if !matches!(motion, Motion::Up | Motion::Down) {
                    self.desired_col = None;
                }
                self.cursor =
                    if self.mode == Mode::Normal { exec.buf.clamp_normal(target) } else { target };
            }
            Command::Operate(op, target) => {
                let (range, linewise) =
                    self.target_range(op, target, count, explicit_count, &exec.buf);
                self.operate(op, range, linewise, exec);
            }
            Command::Simple(simple) => self.simple(simple, count, exec),
        }
    }

    // ── Motions ──────────────────────────────────────────────────────────────

    /// Where `motion` (repeated `count` times) goes from the cursor.
    ///
    /// `for_operator`: `l` and `w` may then reach the line end (e.g., `dl` on the last char).
    fn motion_target(
        &mut self,
        motion: Motion,
        count: usize,
        explicit_count: bool,
        buf: &Buffer,
        for_operator: bool,
    ) -> (usize, Kind) {
        let cur = self.cursor;
        let line_end = buf.line_end(cur);

        match motion {
            Motion::Left => (cur.saturating_sub(count).max(buf.line_start(cur)), Kind::Exclusive),
            Motion::Right => {
                let limit = if for_operator { line_end } else { buf.line_last_char(cur) };
                ((cur + count).min(limit), Kind::Exclusive)
            }
            Motion::Down | Motion::Up => {
                let col = *self.desired_col.get_or_insert_with(|| cur - buf.line_start(cur));
                let line = buf.line_of(cur);
                let line = if motion == Motion::Down {
                    (line + count).min(buf.line_count() - 1)
                } else {
                    line.saturating_sub(count)
                };
                let start = buf.nth_line_start(line);
                ((start + col).min(buf.line_last_char(start)), Kind::Linewise)
            }
            Motion::WordForward => {
                let mut i = cur;
                for _ in 0..count {
                    i = buf.next_word_start(i);
                }
                // `dw` on the last word of a line must not eat the newline.
                if for_operator && i > line_end {
                    i = line_end;
                }
                (i, Kind::Exclusive)
            }
            Motion::WordBackward => {
                let mut i = cur;
                for _ in 0..count {
                    i = buf.prev_word_start(i);
                }
                (i, Kind::Exclusive)
            }
            Motion::WordEnd => {
                let mut i = cur;
                for _ in 0..count {
                    i = buf.word_end(i);
                }
                (i, Kind::Inclusive)
            }
            Motion::LineStart => (buf.line_start(cur), Kind::Exclusive),
            Motion::FirstNonBlank => (buf.first_non_blank(cur), Kind::Exclusive),
            Motion::LineEnd => {
                // `3$` = end of the line 2 below.
                let line = (buf.line_of(cur) + count - 1).min(buf.line_count() - 1);
                (buf.line_last_char(buf.nth_line_start(line)), Kind::Inclusive)
            }
            Motion::FileStart | Motion::FileEnd => {
                let line = if explicit_count {
                    count - 1
                } else if motion == Motion::FileStart {
                    0
                } else {
                    buf.line_count() - 1
                };
                (buf.first_non_blank(buf.nth_line_start(line)), Kind::Linewise)
            }
        }
    }

    /// Range an operator acts on, plus whether it is linewise.
    fn target_range(
        &mut self,
        op: Operator,
        target: Target,
        count: usize,
        explicit_count: bool,
        buf: &Buffer,
    ) -> (Range<usize>, bool) {
        let cur = self.cursor;
        match target {
            Target::Line => {
                let last_line = (buf.line_of(cur) + count - 1).min(buf.line_count() - 1);
                (cur..buf.nth_line_start(last_line), true)
            }
            Target::Word { around } => (buf.word_object(cur, around), false),
            Target::Motion(motion) => {
                // Vim quirk: `cw` on a word behaves like `ce` (keeps the following space).
                let on_word = buf.chars.get(cur).is_some_and(|c| !c.is_whitespace());
                let motion = if op == Operator::Change && motion == Motion::WordForward && on_word {
                    Motion::WordEnd
                } else {
                    motion
                };

                let (target, kind) = self.motion_target(motion, count, explicit_count, buf, true);
                let (lo, hi) = (cur.min(target), cur.max(target));
                match kind {
                    Kind::Exclusive => (lo..hi, false),
                    Kind::Inclusive => (lo..(hi + 1).min(buf.len()), false),
                    Kind::Linewise => (lo..hi, true),
                }
            }
        }
    }

    // ── Operators ────────────────────────────────────────────────────────────

    /// `range` for linewise operators only marks the first/last line; whole lines are used.
    fn operate(&mut self, op: Operator, range: Range<usize>, linewise: bool, exec: &mut Exec) {
        let buf = &exec.buf;
        let (start, end) = if linewise {
            (buf.line_start(range.start), buf.line_end(range.end))
        } else {
            (range.start, range.end)
        };
        if start > end || (start == end && !linewise) {
            return;
        }

        self.yank(buf.slice(start..end), linewise, exec);
        match op {
            Operator::Yank => self.cursor = start,
            Operator::Delete => {
                let delete =
                    if linewise { with_line_break(&exec.buf, start, end) } else { start..end };
                exec.edit(self.cursor, delete.clone(), "");
                let at = delete.start.min(exec.buf.len());
                self.cursor =
                    if linewise { exec.buf.first_non_blank(at) } else { exec.buf.clamp_normal(at) };
            }
            Operator::Change => {
                // `cc` keeps the (now empty) line to type into.
                exec.edit(self.cursor, start..end, "");
                self.cursor = start;
                self.mode = Mode::Insert;
            }
        }
        if op != Operator::Change && matches!(self.mode, Mode::Visual | Mode::VisualLine) {
            self.mode = Mode::Normal;
        }
    }

    fn yank(&mut self, text: String, linewise: bool, exec: &Exec) {
        exec.ctx.copy_text(text.clone());
        self.register = Register { text, linewise };
    }

    /// In Visual mode, `d`/`x`, `y`, `c`/`s` act on the selection directly.
    /// Returns `false` if `key` is not such an operator.
    pub(super) fn visual_operator(&mut self, key: VimKey, exec: &mut Exec) -> bool {
        let op = match key {
            VimKey::Char('d' | 'x') => Operator::Delete,
            VimKey::Char('y') => Operator::Yank,
            VimKey::Char('c' | 's') => Operator::Change,
            _ => return false,
        };
        let (lo, hi) = (self.anchor.min(self.cursor), self.anchor.max(self.cursor));
        let linewise = self.mode == Mode::VisualLine;
        let range = if linewise { lo..hi } else { lo..(hi + 1).min(exec.buf.len()) };
        self.cursor = lo;
        self.operate(op, range, linewise, exec);
        if op == Operator::Yank {
            self.mode = Mode::Normal;
        }
        true
    }

    // ── Simple commands ──────────────────────────────────────────────────────

    fn simple(&mut self, simple: Simple, count: usize, exec: &mut Exec) {
        let cur = self.cursor;
        let (line_start, line_end) = (exec.buf.line_start(cur), exec.buf.line_end(cur));

        match simple {
            Simple::DeleteChar => {
                self.operate(Operator::Delete, cur..(cur + count).min(line_end), false, exec);
            }
            Simple::DeleteCharBefore => {
                self.operate(
                    Operator::Delete,
                    cur.saturating_sub(count).max(line_start)..cur,
                    false,
                    exec,
                );
            }
            Simple::DeleteToEnd => self.operate(Operator::Delete, cur..line_end, false, exec),
            Simple::ChangeToEnd => self.operate(Operator::Change, cur..line_end, false, exec),
            Simple::YankLine => {
                let (range, _) =
                    self.target_range(Operator::Yank, Target::Line, count, false, &exec.buf);
                self.operate(Operator::Yank, range, true, exec);
            }
            Simple::PasteAfter => self.paste(true, count, exec),
            Simple::PasteBefore => self.paste(false, count, exec),
            Simple::Insert => self.mode = Mode::Insert,
            Simple::Append => self.insert_at((cur + 1).min(line_end)),
            Simple::InsertLineStart => self.insert_at(exec.buf.first_non_blank(cur)),
            Simple::AppendLineEnd => self.insert_at(line_end),
            Simple::OpenBelow => {
                exec.edit(cur, line_end..line_end, "\n");
                self.insert_at(line_end + 1);
            }
            Simple::OpenAbove => {
                exec.edit(cur, line_start..line_start, "\n");
                self.insert_at(line_start);
            }
            Simple::JoinLines => self.join_lines(count, exec),
            Simple::ToggleCase => {
                let end = (cur + count).min(line_end);
                let toggled: String = exec.buf.chars[cur..end]
                    .iter()
                    .flat_map(|c| {
                        if c.is_uppercase() {
                            c.to_lowercase().collect::<Vec<_>>()
                        } else {
                            c.to_uppercase().collect()
                        }
                    })
                    .collect();
                exec.edit(cur, cur..end, &toggled);
                self.cursor = exec.buf.clamp_normal(end);
            }
            Simple::Replace(c) => {
                if cur + count <= line_end {
                    exec.edit(cur, cur..cur + count, &c.to_string().repeat(count));
                    self.cursor = cur + count - 1;
                }
            }
            Simple::Undo => self.undo_redo(true, exec),
            Simple::Redo => self.undo_redo(false, exec),
            Simple::Visual | Simple::VisualLine => {
                let mode = if simple == Simple::Visual { Mode::Visual } else { Mode::VisualLine };
                // Pressing `v` again in Visual mode leaves it.
                self.mode = if self.mode == mode { Mode::Normal } else { mode };
                self.anchor = cur;
            }
            Simple::CommandLine => self.command_line = Some(String::new()),
        }
    }

    const fn insert_at(&mut self, at: usize) {
        self.cursor = at;
        self.mode = Mode::Insert;
    }

    /// `p` / `P`. Linewise text goes on its own line; charwise text after/before the cursor.
    fn paste(&mut self, after: bool, count: usize, exec: &mut Exec) {
        if self.register.text.is_empty() && !self.register.linewise {
            return;
        }
        let cur = self.cursor;

        if self.register.linewise {
            let lines = vec![self.register.text.as_str(); count].join("\n");
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
            let text = self.register.text.repeat(count);
            let line_empty = exec.buf.line_start(cur) == exec.buf.line_end(cur);
            let at = if after && !line_empty { cur + 1 } else { cur };
            exec.edit(cur, at..at, &text);
            self.cursor = at + text.chars().count().saturating_sub(1);
        }
    }

    /// `J`: joins the next line(s) with a single space.
    fn join_lines(&mut self, count: usize, exec: &mut Exec) {
        exec.snapshot(self.cursor);
        for _ in 0..count.max(2) - 1 {
            let newline = exec.buf.line_end(self.cursor);
            if newline >= exec.buf.len() {
                break;
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

    /// `u` / `Ctrl+R` through the `TextEdit`'s undo history.
    fn undo_redo(&mut self, undo: bool, exec: &mut Exec) {
        let current = (CCursorRange::one(CCursor::new(self.cursor)), exec.buf.text());
        let restored = if undo { exec.undoer.undo(&current) } else { exec.undoer.redo(&current) };
        if let Some((range, text)) = restored.cloned() {
            exec.buf = Buffer::new(&text);
            self.cursor = exec.buf.clamp_normal(range.primary.index.0);
            exec.output.changed = true;
        }
    }
}

/// Lines `start..end` plus one line break, so deleting them removes the lines entirely.
/// On the last line the break *before* it is taken instead.
const fn with_line_break(buf: &Buffer, start: usize, end: usize) -> Range<usize> {
    if end < buf.len() { start..end + 1 } else { start.saturating_sub(1)..end }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::code_editor::vim::command::{Parse, parse};

    /// Runs `keys` (Normal mode) on `text` with the cursor at `cursor`.
    fn run(text: &str, cursor: usize, keys: &str) -> (String, usize, Mode) {
        let mut vim = VimState { cursor, ..VimState::default() };
        let mut exec = Exec {
            buf: Buffer::new(text),
            undoer: TextEditUndoer::default(),
            ctx: egui::Context::default(),
            output: VimOutput::default(),
        };
        for c in keys.chars() {
            vim.pending.push(VimKey::Char(c));
            if let Parse::Done { command, count, explicit_count } = parse(&vim.pending) {
                vim.pending.clear();
                vim.execute(command, count, explicit_count, &mut exec);
            } else if vim.mode != Mode::Normal {
                vim.pending.clear();
            }
        }
        (exec.buf.text(), vim.cursor, vim.mode)
    }

    #[test]
    fn delete_line_and_paste() {
        let (text, cursor, _) = run("a\nb\nc", 0, "ddp");
        assert_eq!(text, "b\na\nc");
        assert_eq!(cursor, 2);
    }

    #[test]
    fn delete_word_keeps_newline() {
        assert_eq!(run("0.5 foo\nbar", 4, "dw").0, "0.5 \nbar");
    }

    #[test]
    fn change_inner_word() {
        let (text, cursor, mode) = run("0.5 animmotion 1", 6, "ciw");
        assert_eq!(text, "0.5  1");
        assert_eq!((cursor, mode), (4, Mode::Insert));
    }

    #[test]
    fn counts_and_join() {
        assert_eq!(run("a\nb\nc\nd", 0, "2dd").0, "c\nd");
        assert_eq!(run("a\n  b", 0, "J").0, "a b");
        assert_eq!(run("abc", 0, "2x").0, "c");
    }
}
