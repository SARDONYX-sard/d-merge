//! Insert-mode keys Vim handles itself (typing is done by the `TextEdit`):
//!
//! ```text
//! Esc, Ctrl+[, Ctrl+C, jk (see VimOptions)   back to Normal mode
//! Ctrl+W                                     delete the word before the cursor
//! Ctrl+U                                     delete from the line start to the cursor
//! Ctrl+T / Ctrl+D                            indent / outdent the line
//! ```

use super::{Mode, VimState, exec::Exec, keys::VimKey};

impl VimState {
    pub(super) fn feed_insert(&mut self, key: VimKey, exec: &mut Exec) {
        let cur = self.cursor;
        let line_start = exec.buf.line_start(cur);
        match key {
            VimKey::Esc => self.leave_insert(exec),
            VimKey::SequenceEsc(second) => {
                // e.g., `jk`: `j` was typed earlier and is in the text; remove it, then leave.
                // (Checked: the user may have clicked elsewhere in between.)
                if let Some(prev) = cur.checked_sub(1)
                    && exec.buf.chars.get(prev).is_some_and(|&first| {
                        self.options.insert_escapes.contains(&[first, second])
                    })
                {
                    exec.edit(cur, prev..cur, "");
                    self.cursor = prev;
                }
                self.leave_insert(exec);
            }
            VimKey::Ctrl('w') => {
                // At the line start it joins with the previous line, like a backspace.
                let start = if cur == line_start {
                    cur.saturating_sub(1)
                } else {
                    exec.buf.prev_word_start(cur, false).max(line_start)
                };
                exec.edit(cur, start..cur, "");
                self.cursor = start;
            }
            VimKey::Ctrl('u') => {
                exec.edit(cur, line_start..cur, "");
                self.cursor = line_start;
            }
            VimKey::Ctrl(c @ ('t' | 'd')) => {
                let before = exec.buf.line_end(cur) - line_start;
                let line = exec.buf.line_of(cur);
                self.shift_lines(line, line, c == 't', exec);
                let after = exec.buf.line_end(line_start) - line_start;
                self.cursor = if after >= before {
                    cur + (after - before)
                } else {
                    cur.saturating_sub(before - after).max(line_start)
                };
            }
            _ => {}
        }
    }

    /// Back to Normal mode. Like Vim, the cursor steps back onto the last typed char.
    pub(super) fn leave_insert(&mut self, exec: &mut Exec) {
        let typed = self.on_insert_left(exec);
        self.finish_block_insert(typed.as_deref(), exec);
        self.last_insert = Some(self.cursor);
        let start = exec.buf.line_start(self.cursor);
        self.cursor = self.cursor.saturating_sub(1).max(start);
        self.mode = Mode::Normal;
    }
}
