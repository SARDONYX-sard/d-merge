//! Vim key bindings for [`crate::CodeEditor`] (`Keymap::Vim`).
//!
//! # How it plugs into egui
//! Each frame, **before** the `TextEdit` is shown, [`VimState::handle`]:
//! 1. takes the key events Vim owns out of egui's input ([`keys`]),
//! 2. runs them as Vim commands on the text ([`parse`] parses, [`exec`] executes),
//! 3. writes the new cursor/selection into the `TextEdit`'s stored state.
//!
//! The `TextEdit` then draws the result and never sees the swallowed keys.
//! In Insert mode only a few keys are taken ([`insert`]), so typing is plain egui editing
//! (with completion etc.).
//!
//! # Supported subset
//! - modes: Normal, Insert, Visual (`v`), Visual Line (`V`), prompts (`:`, `/`, `?`)
//! - keys: see [`command`] (grammar), [`insert`], [`prompt`] (search), [`ex`] (`:s`, `:sort`, ...)
//! - `.` and macros ([`repeat`]), registers ([`registers`]), marks and jumps ([`marks`])
//! - undo/redo ([`history`]); yanks also go to the system clipboard
//!
//! # Modules
//! - input: [`keys`] -> [`parse`] ([`command`] types)
//! - running: [`exec`] (dispatch), [`motion`], [`edit`], [`textobj`], [`insert`], [`prompt`], [`ex`]
//! - state: [`buffer`], [`history`], [`registers`], [`marks`], [`repeat`]

mod block;
mod buffer;
mod command;
mod edit;
mod ex;
mod exec;
mod history;
mod insert;
mod keys;
mod marks;
mod motion;
mod parse;
mod prompt;
mod registers;
mod repeat;
mod textobj;

use egui::text::{CCursor, CCursorRange};

use self::{
    buffer::Buffer,
    command::{Command, Simple},
    history::History,
    keys::VimKey,
    marks::Marks,
    motion::Find,
    parse::{Parse, parse},
    prompt::{LastSearch, Prompt},
    registers::Registers,
    repeat::{Change, Repeat},
};
use super::{KeymapOutput, Scroll, StatusLine, StatusMode, View};

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
    /// Blockwise visual (`Ctrl+V`), see [`block`].
    VisualBlock,
}

impl Mode {
    pub(crate) const fn is_visual(self) -> bool {
        matches!(self, Self::Visual | Self::VisualLine | Self::VisualBlock)
    }
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
    /// Spaces per `>` / `<` / `Ctrl+T` (Vim's `shiftwidth`). Default: 4.
    pub shift_width: usize,
}

impl Default for VimOptions {
    fn default() -> Self {
        Self {
            insert_escapes: vec![['j', 'k'], ['k', 'j'], ['j', 'j']],
            escape_timeout: 0.5,
            shift_width: 4,
        }
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
    registers: Registers,
    /// The `"x` of the command being run.
    active_register: Option<char>,
    /// `Some` while typing after `:`, `/` or `?`.
    prompt: Option<Prompt>,
    /// The last `/` / `?` search (for `n`, `N` and highlighting).
    search: Option<LastSearch>,
    /// Highlight the matches of `search` (turned off by `:noh`).
    hlsearch: bool,
    /// The last `f` / `t` / `F` / `T` (for `;` and `,`).
    last_find: Option<Find>,
    /// One-shot status message, e.g. `Pattern not found: foo` (cleared by the next key).
    message: Option<String>,
    /// Column kept while moving with `j`/`k` through shorter lines.
    desired_col: Option<usize>,
    /// Cursor and the other end of the selection (Visual modes).
    cursor: usize,
    anchor: usize,
    /// The last Visual selection (`gv`, `:'<,'>`): (mode, anchor, cursor).
    last_visual: Option<(Mode, usize, usize)>,
    /// Where Insert mode was left last time (`gi`).
    last_insert: Option<usize>,
    /// Visual block `I` / `A` / `c`: the other lines to copy the typed text to.
    block_insert: Option<block::BlockInsert>,
    marks: Marks,
    repeat: Repeat,
    /// Undo history (`u`, `Ctrl+R`).
    history: History,
    /// The text as Vim last saw it; if it differs now, it was edited outside Vim
    /// (Insert-mode typing, a quick fix, ...) and that edit gets an undo point.
    known_text: Option<String>,
    /// Selection we stored last frame; if the `TextEdit` reports another one, the mouse moved it.
    last_range: Option<CCursorRange>,
    /// The visible lines (for `H`, `M`, `L`, `Ctrl+D`, ...).
    view: View,
    /// A scroll requested by the keys of this frame (e.g., `zz`).
    scroll: Option<Scroll>,
}

impl VimState {
    pub(crate) const fn mode(&self) -> Mode {
        self.mode
    }

    /// Status line: the mode badge and the details next to it.
    /// e.g., (`Insert`, ``), (`Command`, `:w`), (`Normal`, `recording @a  2d`)
    pub(crate) fn status(&self) -> StatusLine {
        let mode = match self.mode {
            _ if self.prompt.is_some() => StatusMode::Command,
            Mode::Normal => StatusMode::Normal,
            Mode::Insert => StatusMode::Insert,
            Mode::Visual => StatusMode::Visual,
            Mode::VisualLine => StatusMode::VisualLine,
            Mode::VisualBlock => StatusMode::VisualBlock,
        };
        if let Some(prompt) = &self.prompt {
            return StatusLine { mode, text: prompt.display() };
        }
        if let Some(message) = &self.message {
            return StatusLine { mode, text: message.clone() };
        }
        let recording = self
            .repeat
            .recording
            .as_ref()
            .map(|(name, _)| format!("recording @{name}  "))
            .unwrap_or_default();
        let pending: String = self
            .pending
            .iter()
            .filter_map(|k| match k {
                VimKey::Char(c) => Some(*c),
                _ => None,
            })
            .collect();
        StatusLine { mode, text: format!("{recording}{pending}") }
    }
    /// Char index of the block cursor (Normal / Visual modes), if any.
    pub(crate) fn block_cursor(&self, current: Option<usize>, text: &str) -> Option<usize> {
        match self.mode {
            Mode::Insert => None,
            Mode::Visual | Mode::VisualLine | Mode::VisualBlock => Some(self.cursor),
            Mode::Normal => current.map(|i| Buffer::new(text).clamp_normal(i)),
        }
    }

    /// Runs this frame's keys. Call **before** showing the `TextEdit` with id `id`.
    pub(super) fn handle(
        &mut self,
        ui: &egui::Ui,
        id: egui::Id,
        text: &mut String,
        view: View,
    ) -> KeymapOutput {
        self.view = view;
        let inserting = self.mode == Mode::Insert;
        if !inserting {
            self.escape_seq.reset();
        }
        let keys = keys::take(ui, inserting, &mut self.escape_seq, &self.options);
        if keys.is_empty() {
            return KeymapOutput::default();
        }

        self.message = None;
        let ctx = ui.ctx();
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        let current = state.cursor.char_range();

        // Our own cursor is authoritative unless the user clicked somewhere else.
        if current != self.last_range {
            self.cursor = current.map_or(0, |r| r.primary.index.0);
            if self.mode.is_visual() {
                self.mode = Mode::Normal;
            }
        }

        // An edit made outside Vim since the last key (e.g., a whole Insert session): one undo point.
        if let Some(known) = self.known_text.take()
            && known != *text
        {
            self.history.record(known, self.cursor);
        }

        let mut exec = exec::Exec {
            buf: Buffer::new(text),
            history: std::mem::take(&mut self.history),
            ctx: ctx.clone(),
            output: KeymapOutput::default(),
            edits: 0,
        };
        for key in keys {
            self.feed(key, &mut exec);
        }

        if exec.output.changed {
            *text = exec.buf.text();
        }
        self.history = exec.history;
        self.known_text = Some(text.clone());

        self.cursor = self.cursor.min(exec.buf.len());
        let range = self.selection(&exec.buf);
        state.cursor.set_char_range(Some(range));
        state.store(ctx, id);
        // e.g., `G` / `n`: the `TextEdit` only scrolls for moves it makes itself.
        let moved = self.last_range != Some(range);
        exec.output.scroll = self.scroll.take().or_else(|| moved.then_some(Scroll::IntoView));
        self.last_range = Some(range);

        exec.output
    }

    /// Selection to show in the `TextEdit` for the current mode.
    ///
    /// The `primary` end is on the cursor's side: the editor scrolls to it (e.g., `Vk` going up).
    fn selection(&self, buf: &Buffer) -> CCursorRange {
        let (lo, hi) = (self.anchor.min(self.cursor), self.anchor.max(self.cursor));
        let cursor_first = self.cursor < self.anchor;
        let range = |start: usize, end: usize| {
            let mut range = CCursorRange::two(CCursor::new(start), CCursor::new(end));
            if cursor_first {
                std::mem::swap(&mut range.primary, &mut range.secondary);
            }
            range
        };
        match self.mode {
            Mode::Visual => range(lo, (hi + 1).min(buf.len())),
            Mode::VisualLine => range(buf.line_start(lo), buf.line_end(hi)),
            // The block is drawn by the editor (see `block_ranges`); egui has only one selection.
            Mode::Normal | Mode::Insert | Mode::VisualBlock => {
                CCursorRange::one(CCursor::new(self.cursor))
            }
        }
    }

    /// One key.
    fn feed(&mut self, key: VimKey, exec: &mut exec::Exec) {
        let in_command = self.prompt.is_none() && self.mode != Mode::Insert;
        // `q` ends a recording (and is not part of it).
        if in_command
            && self.pending.is_empty()
            && key == VimKey::Char('q')
            && self.repeat.recording.is_some()
        {
            self.repeat.stop_recording();
            return;
        }
        if self.mode != Mode::Insert {
            self.repeat.record_key(key);
        }

        if self.prompt.is_some() {
            self.feed_prompt(key, exec);
        } else if self.mode == Mode::Insert {
            self.feed_insert(key, exec);
        } else {
            self.feed_command(key, exec);
        }

        if self.mode.is_visual() {
            self.last_visual = Some((self.mode, self.anchor, self.cursor));
        }
    }

    /// Normal / Visual mode.
    fn feed_command(&mut self, key: VimKey, exec: &mut exec::Exec) {
        if key == VimKey::Esc {
            self.pending.clear();
            self.mode = Mode::Normal;
            return;
        }
        if self.mode.is_visual() && self.visual_operator(key, exec) {
            self.start_insert_capture(Mode::Visual, exec);
            return;
        }

        self.pending.push(key);
        match parse(&self.pending) {
            Parse::Incomplete => {}
            Parse::Invalid => self.pending.clear(),
            Parse::Done { command, count, explicit_count, register } => {
                let keys = std::mem::take(&mut self.pending);
                let (mode_before, edits_before) = (self.mode, exec.edits);
                self.active_register = register;
                self.execute(command, count, explicit_count, exec);
                self.active_register = None;

                // Remember the change for `.` (only Normal-mode commands, like Vim).
                let repeatable = mode_before == Mode::Normal
                    && !self.repeat.replaying()
                    && !matches!(
                        command,
                        Command::Simple(
                            Simple::Repeat | Simple::Play(_) | Simple::Undo | Simple::Redo
                        )
                    );
                if repeatable {
                    if self.mode == Mode::Insert {
                        self.repeat.pending_change = Some(keys);
                    } else if exec.edits != edits_before {
                        self.repeat.last_change = Some(Change { keys, inserted: None });
                    }
                }
                self.start_insert_capture(mode_before, exec);
            }
        }
    }

    /// Entering Insert mode starts capturing the typed text (for `.` and macros).
    fn start_insert_capture(&mut self, mode_before: Mode, exec: &exec::Exec) {
        if self.mode == Mode::Insert && mode_before != Mode::Insert {
            self.repeat.insert_start = Some(exec.buf.text());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec(text: &str) -> exec::Exec {
        exec::Exec {
            buf: Buffer::new(text),
            history: History::default(),
            ctx: egui::Context::default(),
            output: KeymapOutput::default(),
            edits: 0,
        }
    }

    fn feed_str(vim: &mut VimState, exec: &mut exec::Exec, keys: &str) {
        for c in keys.chars() {
            let key = match c {
                '\n' => VimKey::Enter,
                '\u{1b}' => VimKey::Esc,
                c => VimKey::Char(c),
            };
            vim.feed(key, exec);
        }
    }

    /// Simulates typing in Insert mode (done by the `TextEdit`, not by Vim).
    fn type_text(vim: &mut VimState, exec: &mut exec::Exec, text: &str) {
        exec.buf.replace(vim.cursor..vim.cursor, text);
        vim.cursor += text.chars().count();
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
    fn search_and_repeat() {
        let mut vim = VimState::default();
        let mut exec = exec("ab ab\nab");
        feed_str(&mut vim, &mut exec, "/ab\n");
        assert_eq!(vim.cursor, 3);
        assert!(vim.search_highlight().is_some());
        assert_eq!(vim.status().text, "/ab    [2/3]");
        feed_str(&mut vim, &mut exec, "n");
        assert_eq!(vim.cursor, 6);
        feed_str(&mut vim, &mut exec, "n");
        assert_eq!(vim.cursor, 0); // wraps around
        feed_str(&mut vim, &mut exec, "N");
        assert_eq!(vim.cursor, 6);

        feed_str(&mut vim, &mut exec, ":noh\n");
        assert!(vim.search_highlight().is_none());

        feed_str(&mut vim, &mut exec, "gg*");
        assert_eq!(vim.cursor, 3);
    }

    #[test]
    fn undo_redo_every_step() {
        let mut vim = VimState::default();
        let mut exec = exec("a\nb\nc\nd");
        feed_str(&mut vim, &mut exec, "dddddd");
        assert_eq!(exec.buf.text(), "d");

        feed_str(&mut vim, &mut exec, "uuu");
        assert_eq!(exec.buf.text(), "a\nb\nc\nd");
        feed_str(&mut vim, &mut exec, "u"); // nothing older
        assert_eq!(exec.buf.text(), "a\nb\nc\nd");

        for expected in ["b\nc\nd", "c\nd", "d"] {
            vim.feed(VimKey::Ctrl('r'), &mut exec);
            assert_eq!(exec.buf.text(), expected);
        }
    }

    #[test]
    fn dot_repeats_change_with_inserted_text() {
        let mut vim = VimState::default();
        let mut exec = exec("a b c");
        feed_str(&mut vim, &mut exec, "cw");
        type_text(&mut vim, &mut exec, "X");
        feed_str(&mut vim, &mut exec, "\u{1b}w.");
        assert_eq!(exec.buf.text(), "X X c");

        feed_str(&mut vim, &mut exec, "0x.");
        assert_eq!(exec.buf.text(), "X c");
    }

    #[test]
    fn macro_records_and_plays() {
        let mut vim = VimState::default();
        let mut exec = exec("1\n2\n3");
        feed_str(&mut vim, &mut exec, "qaA");
        type_text(&mut vim, &mut exec, ";");
        feed_str(&mut vim, &mut exec, "\u{1b}jq2@a");
        assert_eq!(exec.buf.text(), "1;\n2;\n3;");
    }

    #[test]
    fn registers_marks_and_visual() {
        let mut vim = VimState::default();
        let mut exec = exec("one\ntwo");
        feed_str(&mut vim, &mut exec, "\"ayyjma\"ap");
        assert_eq!(exec.buf.text(), "one\ntwo\none");
        feed_str(&mut vim, &mut exec, "gg'a");
        assert_eq!(vim.cursor, 4);

        feed_str(&mut vim, &mut exec, "ggvlU");
        assert_eq!(exec.buf.text(), "ONe\ntwo\none");
        feed_str(&mut vim, &mut exec, "v\u{1b}");
        assert_eq!(vim.mode, Mode::Normal);
    }

    #[test]
    fn ex_substitute_and_sort() {
        let mut vim = VimState::default();
        let mut exec = exec("b a\na a\nc");
        feed_str(&mut vim, &mut exec, ":%s/a/x/g\n");
        assert_eq!(exec.buf.text(), "b x\nx x\nc");
        feed_str(&mut vim, &mut exec, ":sort\n");
        assert_eq!(exec.buf.text(), "b x\nc\nx x");
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
