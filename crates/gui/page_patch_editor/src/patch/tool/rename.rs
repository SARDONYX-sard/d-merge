//! Renaming the mod code of the open work, as a [`Job`]: the renaming itself is [`workspace::rename`].
//!
//! ```text
//! [Rename…] -> [newc] [Rename] [Cancel]   (Rename while the code is free and valid)
//!   -> a job: Workspace::rename_code          the editor shows the progress, and nothing else
//!   -> back: saved under newc, the cache of oldc removed, the recent works updated
//! ```
//! Until it is back the work is not here: see [`crate::patch::tool::job`] for why nothing saves.

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};
use nemesis_diff::is_valid_mod_code;

use super::job::{Done, Job};
use crate::patch::{
    PatchTool,
    workspace::{self, Workspace},
};

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
        let Some(old) = self.work.place.loaded_for.clone() else { return };
        if rename_problem(&old, new, |code| Workspace::exists(&self.work.place.cache_root, code)).is_some() {
            return;
        }
        self.work.mark_dirty();
        self.save_now(notify, i18n);
        let mut work = std::mem::take(&mut self.work.workspace);
        let total = workspace::rename::object_count(&work);
        let new = new.trim().to_owned();
        let detail = format!("{old} -> {new}");
        self.work.job = Some(Job::spawn(I18nKey::NemesisRenameCodeRunning, detail, total, move |done| {
            work.rename_code(&old, &new, done);
            Done::Renamed { work, old, new }
        }));
    }

    pub(super) fn finish_rename(
        &mut self,
        work: Workspace,
        old: &str,
        new: &str,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        self.work.workspace = work;
        new.clone_into(&mut self.work.place.mod_code);
        self.work.place.loaded_for = Some(new.to_owned());
        self.reset_names();
        self.tabs.clear();
        self.selected = None;
        self.pane.opened = None;
        self.pane.preview = None;
        self.restore_session();
        self.work.mark_dirty();
        self.save_now(notify, i18n);
        // Only once it is saved under the new code does the old one go.
        if Workspace::exists(&self.work.place.cache_root, new)
            && let Err(err) = std::fs::remove_dir_all(Workspace::cache_dir(&self.work.place.cache_root, old))
        {
            notify.warn(format!("{}: {err}", Workspace::cache_dir(&self.work.place.cache_root, old).display()));
        }
        for work in self.work.recent.iter_mut().filter(|work| work.mod_code.eq_ignore_ascii_case(old)) {
            new.clone_into(&mut work.mod_code);
        }
        self.work.autosave.changed();
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
        let mut tool = PatchTool::default();
        tool.work.place.cache_root.clone_from(&dir);
        let mut behavior =
            workspace::Behavior::open(&std::path::Path::new("meshes").join(format!("{}.bin", workspace::ADSF)))
                .unwrap();
        behavior.added.insert("M~1/Idle~oldc$0.txt".to_owned(), "Idle\noldc$0\n1\n0\n0\n0\n".to_owned());
        tool.work.workspace.behaviors.push(behavior);
        tool.work.place.mod_code = "oldc".to_owned();
        tool.work.place.loaded_for = Some("oldc".to_owned());
        let (mut notify, i18n) = (Notification::default(), I18nMap::default());
        tool.work.mark_dirty();
        tool.save_now(&mut notify, &i18n);
        assert!(Workspace::exists(&dir, "oldc"));

        tool.start_rename("newc", &mut notify, &i18n);
        assert!(tool.work.job.is_some() && tool.work.workspace.behaviors.is_empty());
        tool.wait_job(&mut notify, &i18n);
        let renamed = tool.work.workspace.behaviors[0].added.contains_key("M~1/Idle~newc$0.txt");
        let (old_gone, new_there) = (!Workspace::exists(&dir, "oldc"), Workspace::exists(&dir, "newc"));
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(tool.work.place.mod_code, "newc");
        assert!(renamed && old_gone && new_there);
    }
}
