//! `.` (repeat the last change) and macros (`qa` ... `q`, `@a`, `@@`).
//!
//! # What is replayed
//! Normal-mode keys go through [`VimState::feed`] and are stored as they are. Typing in Insert
//! mode is done by the `TextEdit`, not by Vim, so it is captured differently: when Insert mode
//! is left, the inserted text is computed from the text before and after (see [`inserted_text`])
//! and stored as one [`Step::Insert`].
//!
//! e.g., `ciwfoo<Esc>` is stored as `[c, i, w]` + `Insert("foo")`.

use super::{Mode, VimState, exec::Exec, keys::VimKey};

/// Nested `@a` inside `@a` (or a macro calling itself) stops at this depth.
const MAX_REPLAY_DEPTH: usize = 20;

/// One replayable step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Step {
    Key(VimKey),
    /// Text typed in Insert mode (inserted at the cursor on replay).
    Insert(String),
}

/// The last change, for `.`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Change {
    /// e.g., `[d, w]`, `[c, i, w]`
    pub(super) keys: Vec<VimKey>,
    /// Text typed after the keys, if they entered Insert mode.
    pub(super) inserted: Option<String>,
}

#[derive(Debug, Default)]
pub(super) struct Repeat {
    pub(super) last_change: Option<Change>,
    /// A change that entered Insert mode; completed when Insert mode is left.
    pub(super) pending_change: Option<Vec<VimKey>>,
    /// Text when Insert mode was entered (to find what was typed).
    pub(super) insert_start: Option<String>,
    /// `Some((register, steps))` while recording with `q<c>`.
    pub(super) recording: Option<(char, Vec<Step>)>,
    /// Recorded macros, `a`..`z`.
    macros: Vec<(char, Vec<Step>)>,
    /// The last played macro, for `@@`.
    last_played: Option<char>,
    /// > 0 while replaying (`.` or `@`): nothing is recorded then.
    pub(super) replay_depth: usize,
}

impl Repeat {
    pub(super) const fn replaying(&self) -> bool {
        self.replay_depth > 0
    }

    /// Records a Normal-mode key while `q<c>` is on.
    pub(super) fn record_key(&mut self, key: VimKey) {
        if !self.replaying()
            && let Some((_, steps)) = &mut self.recording
        {
            steps.push(Step::Key(key));
        }
    }

    /// `q` while recording: saves the macro.
    pub(super) fn stop_recording(&mut self) {
        if let Some((name, steps)) = self.recording.take() {
            match self.macros.iter_mut().find(|(c, _)| *c == name) {
                Some((_, slot)) => *slot = steps,
                None => self.macros.push((name, steps)),
            }
        }
    }

    fn macro_steps(&mut self, name: char) -> Option<Vec<Step>> {
        let name = if name == '@' { self.last_played? } else { name.to_ascii_lowercase() };
        self.last_played = Some(name);
        self.macros.iter().find(|(c, _)| *c == name).map(|(_, steps)| steps.clone())
    }
}

impl VimState {
    /// `.`: replays the last change.
    pub(super) fn repeat_last_change(&mut self, exec: &mut Exec) {
        let Some(change) = self.repeat.last_change.clone() else { return };
        let mut steps: Vec<Step> = change.keys.into_iter().map(Step::Key).collect();
        if let Some(text) = change.inserted {
            steps.push(Step::Insert(text));
            steps.push(Step::Key(VimKey::Esc));
        }
        self.replay(&steps, exec);
    }

    /// `@a` (`count` times).
    pub(super) fn play_macro(&mut self, name: char, count: usize, exec: &mut Exec) {
        let Some(steps) = self.repeat.macro_steps(name) else {
            self.message = Some(format!("No macro in register {name}"));
            return;
        };
        for _ in 0..count {
            self.replay(&steps, exec);
        }
    }

    fn replay(&mut self, steps: &[Step], exec: &mut Exec) {
        if self.repeat.replay_depth >= MAX_REPLAY_DEPTH {
            return;
        }
        self.repeat.replay_depth += 1;
        for step in steps {
            match step {
                Step::Key(key) => self.feed(*key, exec),
                Step::Insert(text) if self.mode == Mode::Insert => {
                    exec.edit(self.cursor, self.cursor..self.cursor, text);
                    self.cursor += text.chars().count();
                }
                Step::Insert(_) => {}
            }
        }
        self.repeat.replay_depth -= 1;
    }

    /// Called when Insert mode is left: completes the pending `.` change and the macro step.
    /// Returns the typed text (see [`inserted_text`]).
    pub(super) fn on_insert_left(&mut self, exec: &Exec) -> Option<String> {
        let before = self.repeat.insert_start.take();
        let inserted = before.and_then(|before| inserted_text(&before, &exec.buf.text()));

        if let Some(keys) = self.repeat.pending_change.take() {
            self.repeat.last_change = Some(Change { keys, inserted: inserted.clone() });
        }
        if !self.repeat.replaying()
            && let Some((_, steps)) = &mut self.repeat.recording
        {
            if let Some(text) = &inserted {
                steps.push(Step::Insert(text.clone()));
            }
            steps.push(Step::Key(VimKey::Esc));
        }
        inserted
    }
}

/// The text inserted between `before` and `after`, if `after` is `before` plus one insertion.
///
/// e.g., `"ab"` -> `"aXYb"` = `Some("XY")`; a deletion or a replacement gives `None`.
pub(super) fn inserted_text(before: &str, after: &str) -> Option<String> {
    let (b, a): (Vec<char>, Vec<char>) = (before.chars().collect(), after.chars().collect());
    if a.len() < b.len() {
        return None;
    }
    let prefix = b.iter().zip(&a).take_while(|(x, y)| x == y).count();
    let max_suffix = b.len() - prefix;
    let suffix =
        b.iter().rev().zip(a.iter().rev()).take(max_suffix).take_while(|(x, y)| x == y).count();
    // Everything of `before` must be kept around the insertion.
    (prefix + suffix == b.len()).then(|| a[prefix..a.len() - suffix].iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_insertion() {
        assert_eq!(inserted_text("ab", "aXYb").as_deref(), Some("XY"));
        assert_eq!(inserted_text("aa", "aaa").as_deref(), Some("a"));
        assert_eq!(inserted_text("ab", "ab").as_deref(), Some(""));
        assert_eq!(inserted_text("abc", "ac"), None);
        assert_eq!(inserted_text("abc", "aXc"), None);
    }
}
