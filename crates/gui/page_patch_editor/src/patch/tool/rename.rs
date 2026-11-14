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

use super::{
    job::{Done, Job},
    session::Shown,
    work_state::Work,
};
use crate::patch::workspace::{self, Workspace};

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

impl Work {
    /// Starts renaming the open work to `new`, saved first. See the module docs.
    pub(in crate::patch) fn start_rename(
        &mut self,
        shown: Shown<'_>,
        new: &str,
        notify: &mut Notification,
        i18n: &I18nMap,
    ) {
        let Some(old) = self.place.loaded_for.clone() else { return };
        if rename_problem(&old, new, |code| Workspace::exists(&self.place.cache_root, code)).is_some() {
            return;
        }
        self.mark_dirty();
        self.save_now(shown, notify, i18n);

        let mut work = std::mem::take(&mut self.workspace);
        let total = workspace::rename::object_count(&work);
        let new = new.trim().to_owned();
        let detail = format!("{old} -> {new}");
        self.job = Some(Job::spawn(I18nKey::NemesisRenameCodeRunning, detail, total, move |done| {
            work.rename_code(&old, &new, done);
            Done::Renamed { work, old, new }
        }));
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
        let (tabs, sel) = (crate::patch::editor::tabs::EditorTabs::default(), Default::default());
        let shown = Shown { tabs: &tabs, sel: &sel };
        let mut work = Work::default();
        work.place.cache_root.clone_from(&dir);
        let mut behavior =
            workspace::Behavior::open(&std::path::Path::new("meshes").join(format!("{}.bin", workspace::ADSF)))
                .unwrap();
        behavior.added.insert("M~1/Idle~oldc$0.txt".to_owned(), "Idle\noldc$0\n1\n0\n0\n0\n".to_owned());
        work.workspace.behaviors.push(behavior);
        work.place.mod_code = "oldc".to_owned();
        work.place.loaded_for = Some("oldc".to_owned());
        let (mut notify, i18n) = (Notification::default(), I18nMap::default());
        work.mark_dirty();
        work.save_now(shown, &mut notify, &i18n);
        assert!(Workspace::exists(&dir, "oldc"));

        work.start_rename(shown, "newc", &mut notify, &i18n);
        assert!(work.job.is_some() && work.workspace.behaviors.is_empty());
        work.wait_job(shown, &mut notify, &i18n);
        let renamed = work.workspace.behaviors[0].added.contains_key("M~1/Idle~newc$0.txt");
        let (old_gone, new_there) = (!Workspace::exists(&dir, "oldc"), Workspace::exists(&dir, "newc"));
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(work.place.mod_code, "newc");
        assert!(renamed && old_gone && new_there);
    }
}
