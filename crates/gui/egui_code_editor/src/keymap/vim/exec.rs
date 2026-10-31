//! Runs parsed commands: [`Exec`] (the text being edited) and the command dispatch.
//!
//! The actual work lives in topic modules: motions in [`super::motion`], operators and other
//! edits in [`super::edit`], search in [`super::prompt`], `.` / macros in [`super::repeat`].

use std::ops::Range;

use super::{
    Mode, VimState,
    buffer::Buffer,
    command::{Command, Motion, ObjectKind, Operator, ScrollTo, Simple, Target},
    history::{History, Snapshot},
    motion::Kind,
    prompt::{Prompt, PromptKind},
};
use crate::keymap::{KeymapOutput, Scroll};

/// Mutable context while running a batch of keys.
pub(super) struct Exec {
    pub(super) buf: Buffer,
    /// Vim's undo history (taken out of [`VimState`] while the keys run).
    pub(super) history: History,
    /// For the clipboard and opening URLs.
    pub(super) ctx: egui::Context,
    pub(super) output: KeymapOutput,
    /// Number of edits so far (to tell whether a command changed the text, for `.`).
    pub(super) edits: usize,
}

impl Exec {
    /// Records the state before an edit, so `u` can restore it.
    pub(super) fn snapshot(&mut self, cursor: usize) {
        self.history.record(self.buf.text(), cursor);
        self.edits += 1;
    }

    pub(super) fn edit(&mut self, cursor: usize, range: Range<usize>, with: &str) {
        self.snapshot(cursor);
        self.buf.replace(range, with);
        self.output.changed = true;
    }
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
                let Some((target, _)) =
                    self.motion_target(motion, count, explicit_count, &exec.buf, false)
                else {
                    return;
                };
                if motion.is_jump() {
                    self.marks.push_jump(self.cursor);
                }
                if !matches!(motion, Motion::Up | Motion::Down) {
                    self.desired_col = None;
                }
                self.cursor = if self.mode == Mode::Normal {
                    exec.buf.clamp_normal(target)
                } else {
                    target.min(exec.buf.len())
                };
            }
            Command::Operate(op, target) => {
                if let Some((range, linewise)) =
                    self.target_range(op, target, count, explicit_count, &exec.buf)
                {
                    self.operate(op, range, linewise, exec);
                }
            }
            Command::Simple(simple) => self.simple(simple, count, exec),
        }
    }

    /// Range an operator acts on, plus whether it is linewise. `None`: nothing to do
    /// (e.g., `di(` outside parentheses).
    fn target_range(
        &mut self,
        op: Operator,
        target: Target,
        count: usize,
        explicit_count: bool,
        buf: &Buffer,
    ) -> Option<(Range<usize>, bool)> {
        let cur = self.cursor;
        // `>`/`<` always work on whole lines, like Vim.
        let force_lines = matches!(op, Operator::Indent | Operator::Outdent);
        Some(match target {
            Target::Line => {
                let last_line = (buf.line_of(cur) + count - 1).min(buf.line_count() - 1);
                (cur..buf.nth_line_start(last_line), true)
            }
            Target::Object { kind, around } => {
                let range = match kind {
                    ObjectKind::Word => buf.word_object(cur, around, false),
                    ObjectKind::BigWord => buf.word_object(cur, around, true),
                    ObjectKind::Paragraph => {
                        return Some((buf.paragraph_object(cur, around), true));
                    }
                    ObjectKind::Quote(q) => buf.quote_object(cur, q, around)?,
                    ObjectKind::Pair(open, close) => buf.pair_object(cur, open, close, around)?,
                };
                (range, force_lines)
            }
            Target::Motion(motion) => {
                // Vim quirk: `cw` on a word changes only up to its end (keeps the following
                // space), like `ce` but staying in the current word: `cw` on `a` in `a b` = `a`.
                let on_word = buf.chars.get(cur).is_some_and(|c| !c.is_whitespace());
                let (target, kind) = match motion {
                    Motion::WordForward { big } if op == Operator::Change && on_word => {
                        let mut end = buf.word_object(cur, false, big).end - 1;
                        for _ in 1..count {
                            end = buf.word_end(end, big);
                        }
                        (end, Kind::Inclusive)
                    }
                    _ => self.motion_target(motion, count, explicit_count, buf, true)?,
                };
                let (lo, hi) = (cur.min(target), cur.max(target));
                match kind {
                    _ if force_lines => (lo..hi, true),
                    Kind::Exclusive => (lo..hi, false),
                    Kind::Inclusive => (lo..(hi + 1).min(buf.len()), false),
                    Kind::Linewise => (lo..hi, true),
                }
            }
        })
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
                let start = cur.saturating_sub(count).max(line_start);
                self.operate(Operator::Delete, start..cur, false, exec);
            }
            Simple::DeleteToEnd => self.operate(Operator::Delete, cur..line_end, false, exec),
            Simple::ChangeToEnd => self.operate(Operator::Change, cur..line_end, false, exec),
            Simple::YankLine => {
                if let Some((range, _)) =
                    self.target_range(Operator::Yank, Target::Line, count, false, &exec.buf)
                {
                    self.operate(Operator::Yank, range, true, exec);
                }
            }
            Simple::PasteAfter => self.paste(true, count, exec),
            Simple::PasteBefore => self.paste(false, count, exec),
            Simple::Insert => self.insert_at(cur),
            Simple::Append => self.insert_at((cur + 1).min(line_end)),
            Simple::InsertLineStart => self.insert_at(exec.buf.first_non_blank(cur)),
            Simple::AppendLineEnd => self.insert_at(line_end),
            Simple::InsertAtLastInsert => {
                let at = self.last_insert.unwrap_or(cur).min(exec.buf.len());
                self.insert_at(at);
            }
            Simple::InsertAtColumnZero => self.insert_at(line_start),
            Simple::OpenBelow => {
                exec.edit(cur, line_end..line_end, "\n");
                self.insert_at(line_end + 1);
            }
            Simple::OpenAbove => {
                exec.edit(cur, line_start..line_start, "\n");
                self.insert_at(line_start);
            }
            Simple::JoinLines { spaces } => self.join_lines(count, spaces, exec),
            Simple::ToggleCase => {
                let end = (cur + count).min(line_end);
                self.operate(Operator::ToggleCase, cur..end, false, exec);
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
            Simple::Repeat => self.repeat_last_change(exec),
            Simple::Visual | Simple::VisualLine | Simple::VisualBlock => {
                let mode = match simple {
                    Simple::Visual => Mode::Visual,
                    Simple::VisualLine => Mode::VisualLine,
                    _ => Mode::VisualBlock,
                };
                if !self.mode.is_visual() {
                    self.anchor = cur;
                }
                // The same key again leaves Visual mode; the other one switches the kind.
                self.mode = if self.mode == mode { Mode::Normal } else { mode };
            }
            Simple::Reselect => {
                if let Some((mode, anchor, cursor)) = self.last_visual {
                    let len = exec.buf.len();
                    (self.mode, self.anchor, self.cursor) =
                        (mode, anchor.min(len), cursor.min(len));
                }
            }
            Simple::CommandLine => {
                self.prompt = Some(if self.mode.is_visual() {
                    self.mode = Mode::Normal;
                    Prompt::with_text(PromptKind::Ex, "'<,'>")
                } else {
                    Prompt::new(PromptKind::Ex)
                });
            }
            Simple::SearchForward => self.prompt = Some(Prompt::new(PromptKind::SearchForward)),
            Simple::SearchBackward => self.prompt = Some(Prompt::new(PromptKind::SearchBackward)),
            Simple::SearchWord { forward, whole } => self.search_word(forward, whole, &exec.buf),
            Simple::Hover => exec.output.show_hover = true,
            Simple::OpenUrl => self.open_url(exec),
            Simple::Increment { up } => self.increment(count, up, exec),
            Simple::SetMark(name) => self.marks.set(name, cur),
            Simple::Record(name) => {
                if name.is_ascii_alphanumeric() {
                    self.repeat.recording = Some((name.to_ascii_lowercase(), Vec::new()));
                }
            }
            Simple::Play(name) => self.play_macro(name, count, exec),
            Simple::Jump { older } => {
                if let Some(at) = self.marks.walk(older, cur) {
                    self.cursor = exec.buf.clamp_normal(at.min(exec.buf.len()));
                }
            }
            Simple::ScrollCursor { to, first_non_blank } => {
                if first_non_blank {
                    self.cursor = exec.buf.first_non_blank(cur);
                }
                self.scroll = Some(Scroll::Align(match to {
                    ScrollTo::Center => egui::Align::Center,
                    ScrollTo::Top => egui::Align::Min,
                    ScrollTo::Bottom => egui::Align::Max,
                }));
            }
            Simple::ScrollLines { down } => {
                let lines = i32::try_from(count).unwrap_or(i32::MAX);
                self.scroll = Some(Scroll::Lines(if down { lines } else { -lines }));
            }
            Simple::Nop => {}
        }
    }

    const fn insert_at(&mut self, at: usize) {
        self.cursor = at;
        self.mode = Mode::Insert;
    }

    /// `u` / `Ctrl+R` (see [`History`]).
    pub(super) fn undo_redo(&mut self, undo: bool, exec: &mut Exec) {
        let current = Snapshot { text: exec.buf.text(), cursor: self.cursor };
        let restored = if undo { exec.history.undo(current) } else { exec.history.redo(current) };
        match restored {
            Some(Snapshot { text, cursor }) => {
                exec.buf = Buffer::new(&text);
                self.cursor = exec.buf.clamp_normal(cursor);
                exec.output.changed = true;
            }
            None => {
                self.message = Some(
                    if undo { "Already at oldest change" } else { "Already at newest change" }
                        .to_string(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::parse::{Parse, parse},
        *,
    };
    use crate::keymap::vim::keys::VimKey;

    /// Runs `keys` (Normal mode) on `text` with the cursor at `cursor`.
    fn run(text: &str, cursor: usize, keys: &str) -> (String, usize, Mode) {
        let mut vim = VimState { cursor, ..VimState::default() };
        let mut exec = Exec {
            buf: Buffer::new(text),
            history: History::default(),
            ctx: egui::Context::default(),
            output: KeymapOutput::default(),
            edits: 0,
        };
        for c in keys.chars() {
            vim.pending.push(VimKey::Char(c));
            if let Parse::Done { command, count, explicit_count, .. } = parse(&vim.pending) {
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
        assert_eq!(cursor, 4);
        assert_eq!(mode, Mode::Insert);
    }

    #[test]
    fn text_objects_and_finds() {
        assert_eq!(run("f(a, b) x", 3, "di(").0, "f() x");
        assert_eq!(run("say \"hi there\" ok", 0, "ci\"").0, "say \"\" ok");
        assert_eq!(run("a, b, c", 0, "dt,").0, ", b, c");
        assert_eq!(run("a, b, c", 0, "df,;").0, " b, c");
        assert_eq!(run("a, b, c", 0, "2f,").1, 4);
        assert_eq!(run("(a (b) c)", 0, "%").1, 8);
    }

    #[test]
    fn case_indent_and_numbers() {
        assert_eq!(run("hello world", 0, "gUiw").0, "HELLO world");
        assert_eq!(run("Hello", 0, "g~~").0, "hELLO");
        assert_eq!(run("a\nb", 0, ">j").0, "    a\n    b");
        assert_eq!(run("    a", 0, "<<").0, "a");
    }

    #[test]
    fn paragraphs_and_big_words() {
        assert_eq!(run("a\nb\n\nc", 0, "}").1, 4);
        assert_eq!(run("a.b c", 0, "W").1, 4);
        assert_eq!(run("a\nb\n\nc", 0, "dap").0, "c");
    }
}
