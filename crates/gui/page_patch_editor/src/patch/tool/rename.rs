//! Renaming the mod code of the open work, on another thread with a progress bar. The renaming itself
//! is [`workspace::rename`].
//!
//! ```text
//! [Rename…] -> [newc] [Rename] [Cancel]   (Rename while the code is free and valid)
//!   -> another thread: Workspace::rename_code      the editor shows the progress, and nothing else
//!   -> back: saved under newc, the cache of oldc removed, the recent works updated
//! ```
//! Until it is back the work is not here: nothing may save it, or the empty work would be saved over it.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::JoinHandle,
    time::Instant,
};

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_valid_mod_code;

use crate::patch::{
    PatchTool,
    workspace::{self, Workspace},
};

/// A rename running. See the module docs.
#[derive(Debug)]
pub(in crate::patch) struct Renaming {
    pub(in crate::patch) old: String,
    pub(in crate::patch) new: String,
    /// Objects done, of `total`.
    pub(in crate::patch) done: Arc<AtomicUsize>,
    pub(in crate::patch) total: usize,
    work: JoinHandle<Workspace>,
}

impl Renaming {
    /// How far it is, from 0 to 1.
    pub(in crate::patch) fn progress(&self) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "a ratio for a progress bar")]
        let ratio = self.done.load(Ordering::Relaxed) as f32 / self.total.max(1) as f32;
        ratio.min(1.0)
    }
}

/// Why `new` can not be the work's code instead of `old`, if so. `exists`: is there a work with it?
///
/// e.g. `oldc` -> `newc`: none. -> `ab`: invalid (4-6 letters or digits). -> `OLDC`: the same code.
pub(in crate::patch) fn rename_problem(old: &str, new: &str, exists: impl FnOnce(&str) -> bool) -> Option<I18nKey> {
    let new = new.trim();
    if !is_valid_mod_code(new) || new.eq_ignore_ascii_case(old) {
        return Some(I18nKey::NemesisModCodeInvalid);
    }
    (new.eq_ignore_ascii_case(gui_shared::patch::USER_PATCH_CODE) || exists(new)).then_some(I18nKey::NemesisWorkExists)
}

impl PatchTool {
    /// Starts renaming the open work to `new`, saved first. See the module docs.
    pub(in crate::patch) fn start_rename(&mut self, new: &str, notify: &mut Notification, i18n: &I18nMap) {
        let Some(old) = self.loaded_for.clone() else { return };
        if rename_problem(&old, new, |code| Workspace::exists(&self.cache_root, code)).is_some() {
            return;
        }
        self.mark_dirty();
        self.save_now(notify, i18n);
        let mut work = std::mem::take(&mut self.workspace);
        let total = workspace::rename::object_count(&work);
        let done = Arc::new(AtomicUsize::new(0));
        let new = new.trim().to_owned();
        let work = {
            let (old, new, done) = (old.clone(), new.clone(), Arc::clone(&done));
            std::thread::spawn(move || {
                work.rename_code(&old, &new, &done);
                work
            })
        };
        self.renaming = Some(Renaming { old, new, done, total, work });
    }

    /// Takes the work back once the rename is done, and finishes it. Call each frame.
    pub(in crate::patch) fn poll_rename(&mut self, ctx: &egui::Context, notify: &mut Notification, i18n: &I18nMap) {
        let Some(renaming) = self.renaming.take_if(|renaming| renaming.work.is_finished()) else {
            if self.renaming.is_some() {
                ctx.request_repaint();
            }
            return;
        };
        let Renaming { old, new, work, .. } = renaming;
        if let Ok(work) = work.join() {
            self.finish_rename(work, &old, &new, notify, i18n);
        } else {
            // The work under the old code is as it was saved: read again.
            self.loaded_for = None;
            notify.error(format!("{}: {old}", i18n.t(I18nKey::NemesisRenameCodeFailed)));
        }
    }

    fn finish_rename(&mut self, work: Workspace, old: &str, new: &str, notify: &mut Notification, i18n: &I18nMap) {
        self.workspace = work;
        new.clone_into(&mut self.mod_code);
        self.loaded_for = Some(new.to_owned());
        self.reset_names();
        self.tabs.clear();
        self.selected = None;
        self.editor_for = None;
        self.preview = None;
        self.restore_session();
        self.mark_dirty();
        self.save_now(notify, i18n);
        // Only once it is saved under the new code does the old one go.
        if Workspace::exists(&self.cache_root, new)
            && let Err(err) = std::fs::remove_dir_all(Workspace::cache_dir(&self.cache_root, old))
        {
            notify.warn(format!("{}: {err}", Workspace::cache_dir(&self.cache_root, old).display()));
        }
        for work in self.recent.iter_mut().filter(|work| work.mod_code.eq_ignore_ascii_case(old)) {
            new.clone_into(&mut work.mod_code);
        }
        self.dirty_since.get_or_insert_with(Instant::now);
        notify.success(format!("{}: {old} -> {new}", i18n.t(I18nKey::NemesisRenameCodeDone)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_code_must_be_valid_free_and_another() {
        assert_eq!(rename_problem("oldc", "newc", |_| false), None);
        assert_eq!(rename_problem("oldc", " newc ", |_| false), None);
        assert_eq!(rename_problem("oldc", "ab", |_| false), Some(I18nKey::NemesisModCodeInvalid));
        assert_eq!(rename_problem("oldc", "OLDC", |_| false), Some(I18nKey::NemesisModCodeInvalid));
        assert_eq!(rename_problem("oldc", "newc", |_| true), Some(I18nKey::NemesisWorkExists));
        assert_eq!(rename_problem("oldc", "upatch", |_| false), Some(I18nKey::NemesisWorkExists));
    }

    /// The work goes to another thread and comes back with the new code; meanwhile nothing saves.
    #[test]
    fn a_work_is_renamed_and_saved_under_the_new_code() {
        let dir = std::env::temp_dir().join(format!("d_merge_rename_{}", std::process::id()));
        let mut tool = PatchTool { cache_root: dir.clone(), ..PatchTool::default() };
        let mut behavior =
            workspace::Behavior::open(&std::path::Path::new("meshes").join(format!("{}.bin", workspace::ADSF)))
                .unwrap();
        behavior.added.insert("M~1/Idle~oldc$0.txt".to_owned(), "Idle\noldc$0\n1\n0\n0\n0\n".to_owned());
        tool.workspace.behaviors.push(behavior);
        tool.mod_code = "oldc".to_owned();
        tool.loaded_for = Some("oldc".to_owned());
        let (mut notify, i18n) = (Notification::default(), I18nMap::default());
        tool.mark_dirty();
        tool.save_now(&mut notify, &i18n);
        assert!(Workspace::exists(&dir, "oldc"));

        tool.start_rename("newc", &mut notify, &i18n);
        assert!(tool.renaming.is_some() && tool.workspace.behaviors.is_empty());
        let ctx = egui::Context::default();
        while tool.renaming.is_some() {
            tool.poll_rename(&ctx, &mut notify, &i18n);
        }
        let renamed = tool.workspace.behaviors[0].added.contains_key("M~1/Idle~newc$0.txt");
        let (old_gone, new_there) = (!Workspace::exists(&dir, "oldc"), Workspace::exists(&dir, "newc"));
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(tool.mod_code, "newc");
        assert!(renamed && old_gone && new_there);
    }
}
