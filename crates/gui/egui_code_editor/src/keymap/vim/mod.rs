//! Vim key bindings for [`crate::CodeEditor`] (`Keymap::Vim`).
//!
//! # How it plugs into egui
//! Each frame, **before** the `TextEdit` is shown, [`VimState::handle`]:
//! 1. takes the key events Vim owns out of egui's input ([`keys`]),
//! 2. runs them as Vim commands on the text ([`command`] parses, [`exec`] executes),
//! 3. writes the new cursor/selection into the `TextEdit`'s stored state.
//!
//! The `TextEdit` then draws the result and never sees the swallowed keys.
//! In Insert mode only `Esc` is taken, so typing is plain egui editing (with completion etc.).
//!
//! # Supported subset
//! - modes: Normal, Insert, Visual (`v`), Visual Line (`V`), command line (`:`)
//! - see [`command`] for keys; `:w` saves, `:<n>` jumps to line `n`
//! - undo/redo (`u`, `Ctrl+R`) share egui's `TextEdit` undo history
//! - yanks also go to the system clipboard

mod buffer;
mod command;
mod exec;
mod keys;

use egui::text::{CCursor, CCursorRange};

use self::{
    buffer::Buffer,
    command::{Parse, parse},
    keys::VimKey,
};
use super::KeymapOutput;
use crate::EditorCommand;

/// Current Vim mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Normal,
    Insert,
    /// Characterwise visual (`v`).
    Visual,
    /// Linewise visual (`V`).
    VisualLine,
}

/// Unnamed register (`""`): the last yanked/deleted text.
#[derive(Debug, Clone, Default)]
struct Register {
    text: String,
    /// `true` for whole lines (`yy`, `dd`): pasted on its own line.
    linewise: bool,
}

/// User-configurable Vim behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct VimOptions {
    /// Two-char sequences that leave Insert mode like `Esc`.
    ///
    /// e.g., `[['j', 'k']]` = Vim's `inoremap jk <Esc>`. Default: `jk`, `kj`, `jj`.
    pub insert_escapes: Vec<[char; 2]>,
    /// Max seconds between the two chars (like Vim's `timeoutlen`).
    /// Slower typing inserts both chars normally, so e.g. `jj` can still be typed.
    pub escape_timeout: f64,
}

impl Default for VimOptions {
    fn default() -> Self {
        Self { insert_escapes: vec![['j', 'k'], ['k', 'j'], ['j', 'j']], escape_timeout: 0.5 }
    }
}

/// Per-editor Vim state.
#[derive(Debug, Default)]
pub(crate) struct VimState {
    pub(crate) options: VimOptions,
    escape_seq: keys::InsertEscape,
    mode: Mode,
    /// Keys of an unfinished command, e.g., `2d` waiting for a motion.
    pending: Vec<VimKey>,
    register: Register,
    /// `Some` while typing an Ex command after `:`.
    command_line: Option<String>,
    /// Column kept while moving with `j`/`k` through shorter lines.
    desired_col: Option<usize>,
    /// Cursor and the other end of the selection (Visual modes).
    cursor: usize,
    anchor: usize,
    /// Selection we stored last frame; if the `TextEdit` reports another one, the mouse moved it.
    last_range: Option<CCursorRange>,
}

impl VimState {
    pub(crate) const fn mode(&self) -> Mode {
        self.mode
    }

    /// Status line text. e.g., `-- INSERT --`, `:w`, `2d`
    pub(crate) fn status(&self) -> String {
        if let Some(line) = &self.command_line {
            return format!(":{line}");
        }
        let mode = match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "-- INSERT --",
            Mode::Visual => "-- VISUAL --",
            Mode::VisualLine => "-- VISUAL LINE --",
        };
        let pending: String = self
            .pending
            .iter()
            .filter_map(|k| match k {
                VimKey::Char(c) => Some(*c),
                _ => None,
            })
            .collect();
        format!("{mode}  {pending}")
    }

    /// Char index of the block cursor (Normal / Visual modes), if any.
    pub(crate) fn block_cursor(&self, current: Option<usize>, text: &str) -> Option<usize> {
        match self.mode {
            Mode::Insert => None,
            Mode::Visual | Mode::VisualLine => Some(self.cursor),
            Mode::Normal => current.map(|i| Buffer::new(text).clamp_normal(i)),
        }
    }

    /// Runs this frame's keys. Call **before** showing the `TextEdit` with id `id`.
    pub(super) fn handle(
        &mut self,
        ui: &egui::Ui,
        id: egui::Id,
        text: &mut String,
    ) -> KeymapOutput {
        let inserting = self.mode == Mode::Insert;
        if !inserting {
            self.escape_seq.reset();
        }
        let keys = keys::take(ui, inserting, &mut self.escape_seq, &self.options);
        if keys.is_empty() {
            return KeymapOutput::default();
        }

        let ctx = ui.ctx();
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        let current = state.cursor.char_range();

        // Our own cursor is authoritative unless the user clicked somewhere else.
        if current != self.last_range {
            self.cursor = current.map_or(0, |r| r.primary.index.0);
            if matches!(self.mode, Mode::Visual | Mode::VisualLine) {
                self.mode = Mode::Normal;
            }
        }

        let mut exec = exec::Exec {
            buf: Buffer::new(text),
            undoer: state.undoer(),
            ctx: ctx.clone(),
            output: KeymapOutput::default(),
        };
        for key in keys {
            self.feed(key, &mut exec);
        }

        if exec.output.changed {
            *text = exec.buf.text();
        }

        let range = self.selection(&exec.buf);
        state.cursor.set_char_range(Some(range));
        state.set_undoer(exec.undoer);
        state.store(ctx, id);
        self.last_range = Some(range);

        exec.output
    }

    /// Selection to show in the `TextEdit` for the current mode.
    fn selection(&self, buf: &Buffer) -> CCursorRange {
        let (lo, hi) = (self.anchor.min(self.cursor), self.anchor.max(self.cursor));
        match self.mode {
            Mode::Visual => {
                CCursorRange::two(CCursor::new(lo), CCursor::new((hi + 1).min(buf.len())))
            }
            Mode::VisualLine => {
                CCursorRange::two(CCursor::new(buf.line_start(lo)), CCursor::new(buf.line_end(hi)))
            }
            Mode::Normal | Mode::Insert => CCursorRange::one(CCursor::new(self.cursor)),
        }
    }

    /// One key.
    fn feed(&mut self, key: VimKey, exec: &mut exec::Exec) {
        if self.command_line.is_some() {
            self.feed_command_line(key, exec);
            return;
        }

        match self.mode {
            Mode::Insert => match key {
                VimKey::Esc => self.leave_insert(&exec.buf),
                VimKey::SequenceEsc(second) => {
                    // e.g., `jk`: `j` was typed earlier and is in the text; remove it, then leave.
                    // (Checked: the user may have clicked elsewhere in between.)
                    if let Some(prev) = self.cursor.checked_sub(1)
                        && exec.buf.chars.get(prev).is_some_and(|&first| {
                            self.options.insert_escapes.contains(&[first, second])
                        })
                    {
                        exec.edit(self.cursor, prev..self.cursor, "");
                        self.cursor = prev;
                    }
                    self.leave_insert(&exec.buf);
                }
                _ => {}
            },
            Mode::Normal | Mode::Visual | Mode::VisualLine => {
                if key == VimKey::Esc {
                    self.pending.clear();
                    self.mode = Mode::Normal;
                    return;
                }
                if matches!(self.mode, Mode::Visual | Mode::VisualLine)
                    && self.pending.is_empty()
                    && self.visual_operator(key, exec)
                {
                    return;
                }

                self.pending.push(key);
                match parse(&self.pending) {
                    Parse::Incomplete => {}
                    Parse::Invalid => self.pending.clear(),
                    Parse::Done { command, count, explicit_count } => {
                        self.pending.clear();
                        self.execute(command, count, explicit_count, exec);
                    }
                }
            }
        }
    }

    /// Like Vim, leaving Insert mode steps back onto the last typed char.
    fn leave_insert(&mut self, buf: &Buffer) {
        let start = buf.line_start(self.cursor);
        self.cursor = self.cursor.saturating_sub(1).max(start);
        self.mode = Mode::Normal;
    }

    /// `:` line: type, `Backspace` to edit, `Enter` to run, `Esc` to cancel.
    fn feed_command_line(&mut self, key: VimKey, exec: &mut exec::Exec) {
        let Some(line) = &mut self.command_line else { return };
        match key {
            VimKey::Char(c) => line.push(c),
            VimKey::Backspace => {
                if line.pop().is_none() {
                    self.command_line = None;
                }
            }
            VimKey::Esc => self.command_line = None,
            VimKey::Enter => {
                let line = self.command_line.take().unwrap_or_default();
                self.run_ex(line.trim(), exec);
            }
            _ => {}
        }
    }

    /// `:w`, `:wq`, `:x` -> save; `:<n>` -> go to line `n`.
    fn run_ex(&mut self, line: &str, exec: &mut exec::Exec) {
        match line {
            "w" | "write" | "wq" | "x" => exec.output.command = Some(EditorCommand::Save),
            _ => {
                if let Ok(n) = line.parse::<usize>() {
                    let start = exec.buf.nth_line_start(n.saturating_sub(1));
                    self.cursor = exec.buf.first_non_blank(start);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::vim::exec::TextEditUndoer;

    fn exec(text: &str) -> exec::Exec {
        exec::Exec {
            buf: Buffer::new(text),
            undoer: TextEditUndoer::default(),
            ctx: egui::Context::default(),
            output: KeymapOutput::default(),
        }
    }

    #[test]
    fn sequence_escape_removes_first_char_and_leaves_insert() {
        // `foj` typed, then `k` of `jk` arrives as `SequenceEsc('k')`.
        let mut vim = VimState { mode: Mode::Insert, cursor: 3, ..VimState::default() };
        let mut exec = exec("foj");
        vim.feed(VimKey::SequenceEsc('k'), &mut exec);
        assert_eq!(exec.buf.text(), "fo");
        assert_eq!(vim.mode, Mode::Normal);
        assert_eq!(vim.cursor, 1); // steps back onto `o`, like Vim
    }

    #[test]
    fn insert_escape_timing() {
        let options = VimOptions::default();
        let mut seq = keys::InsertEscape::default();
        assert!(!seq.on_char('j', 0.0, &options));
        assert!(seq.on_char('k', 0.1, &options)); // fast `jk` -> escape
        assert!(!seq.on_char('j', 1.0, &options));
        assert!(!seq.on_char('k', 2.0, &options)); // too slow -> typed normally
    }

    #[test]
    fn shift_k_requests_hover() {
        let mut vim = VimState::default();
        let mut exec = exec("0.5 animmotion");
        vim.feed(VimKey::Char('K'), &mut exec);
        assert!(exec.output.show_hover);
        assert!(!exec.output.changed);
    }
}
