//! Vim's own undo history (`u`, `Ctrl+R`).
//!
//! # Why not egui's `TextEdit` undoer?
//! Its state is `(cursor, text)`: every cursor move (e.g., `j`) counts as a change. That adds
//! undo points that only move the cursor and clears the redo list, so `u` / `Ctrl+R` seemed to
//! go back fewer steps than expected. This history records text changes only, like Vim:
//! - one undo point per Normal-mode edit (`dd`, `x`, `p`, ...)
//! - one undo point per Insert session (`i` ... `Esc`)
//! - edits made outside Vim (e.g., a quick fix, formatting) also get one

/// Undo points kept at most (the oldest are dropped).
const MAX_UNDOS: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub(super) text: String,
    /// Cursor (char index) to restore with the text.
    pub(super) cursor: usize,
}

#[derive(Debug, Default)]
pub(super) struct History {
    undos: Vec<Snapshot>,
    redos: Vec<Snapshot>,
}

impl History {
    /// Records the text **before** a change. Consecutive equal texts are kept once.
    pub(super) fn record(&mut self, text: String, cursor: usize) {
        if self.undos.last().is_some_and(|last| last.text == text) {
            return;
        }
        self.undos.push(Snapshot { text, cursor });
        if self.undos.len() > MAX_UNDOS {
            self.undos.remove(0);
        }
        self.redos.clear(); // a new change makes the undone ones unreachable, like Vim
    }

    /// `u`: the previous text, or `None` at the oldest change.
    pub(super) fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let previous = self.undos.pop()?;
        self.redos.push(current);
        Some(previous)
    }

    /// `Ctrl+R`: the undone text, or `None` if nothing was undone.
    pub(super) fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redos.pop()?;
        self.undos.push(current);
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(text: &str) -> Snapshot {
        Snapshot { text: text.to_string(), cursor: 0 }
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut history = History::default();
        history.record("a".into(), 0);
        history.record("ab".into(), 0);
        history.record("ab".into(), 0); // duplicate: ignored

        assert_eq!(history.undo(snap("abc")), Some(snap("ab")));
        assert_eq!(history.undo(snap("ab")), Some(snap("a")));
        assert_eq!(history.undo(snap("a")), None);
        assert_eq!(history.redo(snap("a")), Some(snap("ab")));
        assert_eq!(history.redo(snap("ab")), Some(snap("abc")));
        assert_eq!(history.redo(snap("abc")), None);
    }

    #[test]
    fn new_change_drops_redos() {
        let mut history = History::default();
        history.record("a".into(), 0);
        assert!(history.undo(snap("ab")).is_some());
        history.record("a".into(), 0);
        assert_eq!(history.redo(snap("ax")), None);
    }
}
