//! When the work is saved: a moment after its last change, as VS Code's `files.autoSave:
//! afterDelay`. What is open (the behavior, the object) is part of the work: going to another one is
//! a change too.
//!
//! ```text
//! an edit, a rename, an export     changed()      -> due SAVE_DELAY later
//! Ctrl+S                           save_soon()    -> due now
//! another behavior / object shown  look_at(..)    -> a change, if a work is open
//! each frame                       is_due()       -> PatchTool::save_now -> take()
//! ```

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

/// How long after the last change the work is saved.
pub(in crate::patch) const SAVE_DELAY: Duration = Duration::from_secs(1);

/// See the module docs.
#[derive(Debug, Default)]
pub(in crate::patch) struct Autosave {
    /// Since when there is something to save; `None`: nothing.
    since: Option<Instant>,
    /// The behavior and object shown when last looked at.
    seen: (usize, Option<Rc<str>>),
}

impl Autosave {
    /// Something changed: it is saved [`SAVE_DELAY`] after the first change not saved yet.
    pub(in crate::patch) fn changed(&mut self) {
        self.since.get_or_insert_with(Instant::now);
    }

    /// `Ctrl+S`: saved on the next frame.
    pub(in crate::patch) fn save_soon(&mut self) {
        self.since = Instant::now().checked_sub(SAVE_DELAY);
    }

    /// Is there something to save?
    #[cfg(test)]
    pub(in crate::patch) const fn is_dirty(&self) -> bool {
        self.since.is_some()
    }

    /// Is it time to save?
    pub(in crate::patch) fn is_due(&self) -> bool {
        self.since.is_some_and(|since| since.elapsed() >= SAVE_DELAY)
    }

    /// It is being saved: returns whether there was something to.
    pub(in crate::patch) const fn take(&mut self) -> bool {
        self.since.take().is_some()
    }

    /// What `since` is, to put back with [`Self::restore`]: around something that must not count as
    /// a change (e.g. a proposal shown, not taken yet).
    pub(in crate::patch) const fn since(&self) -> Option<Instant> {
        self.since
    }

    pub(in crate::patch) const fn restore(&mut self, since: Option<Instant>) {
        self.since = since;
    }

    /// `(behavior, object)` is shown. Another one than last time is a change if `has_work`.
    pub(in crate::patch) fn look_at(&mut self, behavior: usize, object: Option<&Rc<str>>, has_work: bool) {
        if self.seen.0 != behavior || self.seen.1.as_ref() != object {
            self.seen_at(behavior, object);
            if has_work {
                self.changed();
            }
        }
    }

    /// `(behavior, object)` is shown, and that is no change: e.g. the work was just opened there.
    pub(in crate::patch) fn seen_at(&mut self, behavior: usize, object: Option<&Rc<str>>) {
        self.seen = (behavior, object.cloned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_is_due_after_the_delay_and_saved_once() {
        let mut save = Autosave::default();
        assert!(!save.is_dirty() && !save.take());

        save.changed();
        assert!(save.is_dirty() && !save.is_due(), "not yet");
        save.save_soon();
        assert!(save.is_due(), "Ctrl+S");
        assert!(save.take() && !save.is_dirty());
    }

    #[test]
    fn showing_another_object_is_a_change_of_an_open_work() {
        let mut save = Autosave::default();
        let object: Rc<str> = "#0106".into();
        save.look_at(0, Some(&object), false);
        assert!(!save.is_dirty(), "no work open");

        save.look_at(0, Some(&object), true);
        assert!(!save.is_dirty(), "the same one");
        save.look_at(1, Some(&object), true);
        assert!(save.is_dirty());
    }
}
