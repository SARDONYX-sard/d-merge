//! What takes long runs on another thread, while the editor shows how far it is: reading a mod or a
//! work of thousands of patches froze the app for seconds.
//!
//! ```text
//! start: Job::spawn(label, run)       run counts in `done` what it went through (files, objects)
//! each frame: PatchTool::poll_job     not done: the progress is drawn instead of the editor
//!             done: PatchTool::finish what was made goes in, as the same work done at once did
//! ```
//!
//! | Job              | Made by                    | `total`                  |
//! | ---------------- | -------------------------- | ------------------------ |
//! | read a mod       | `Workspace::import_counted` | unknown: files counted   |
//! | load a work      | `Workspace::load_counted`   | unknown: objects counted |
//! | rename its code  | `Workspace::rename_code`    | its objects              |
//!
//! While one runs nothing saves: a rename has taken the work away, and the empty one left would be
//! saved over it.
//!
//! The thread and the counting are [`gui_shared::job::Job`].

use std::sync::atomic::AtomicUsize;

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{PatchTool, workspace::Workspace};

/// What a reload keeps of the work: it is not in the mod. See `PatchTool::reload_from_mod`.
#[derive(Debug)]
pub(in crate::patch) struct Kept {
    pub(in crate::patch) output: String,
    pub(super) refs: Vec<String>,
}

/// What a job made, to take in. See [`PatchTool::finish_job`].
#[derive(Debug)]
pub(in crate::patch) enum Done {
    /// A mod read: `(work, code, folders without a template)`. `source`: the mod's root.
    Read { read: Result<(Workspace, String, Vec<String>), String>, source: String, kept: Option<Kept> },
    /// The work of `code` loaded from its cache.
    Loaded { loaded: Result<Workspace, String>, code: String },
    /// The work with its code renamed from `old` to `new`.
    Renamed { work: Workspace, old: String, new: String },
}

/// A job running, and what the page says of it. See the module docs.
#[derive(Debug)]
pub(in crate::patch) struct Job {
    /// What is shown, e.g. `Renaming the mod code`, and after it `detail`, e.g. `oldc -> newc`.
    pub(in crate::patch) label: I18nKey,
    pub(in crate::patch) detail: String,
    pub(in crate::patch) work: gui_shared::job::Job<Done>,
}

impl Job {
    /// Runs `run` on another thread, counting in the `AtomicUsize` it is given out of `total` (`0`:
    /// not known).
    pub(super) fn spawn(
        label: I18nKey,
        detail: String,
        total: usize,
        run: impl FnOnce(&AtomicUsize) -> Done + Send + 'static,
    ) -> Self {
        Self { label, detail, work: gui_shared::job::Job::spawn(total, run) }
    }
}

impl PatchTool {
    /// Takes in the job once it is done. Returns whether one is still running. Call each frame.
    pub(in crate::patch) fn poll_job(&mut self, notify: &mut Notification, i18n: &I18nMap) -> bool {
        let Some(job) = self.job.take_if(|job| job.work.is_finished()) else { return self.job.is_some() };

        if let Ok(done) = job.work.join() {
            self.finish_job(done, notify, i18n);
        } else {
            // Nothing half done stays: the work is closed, and what is saved of it is as it was.
            notify.error(format!("{}: {}", i18n.t(job.label), job.detail));
            self.close_work(notify, i18n);
        }
        false
    }

    /// Waits for the job, if one runs: where what comes next needs the work, e.g. the user patch,
    /// which is small, or the tests.
    pub(in crate::patch) fn wait_job(&mut self, notify: &mut Notification, i18n: &I18nMap) {
        while self.poll_job(notify, i18n) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn finish_job(&mut self, done: Done, notify: &mut Notification, i18n: &I18nMap) {
        match done {
            Done::Read { read, source, kept } => self.finish_read(read, source, kept, notify, i18n),
            Done::Loaded { loaded, code } => self.finish_load(loaded, code, notify, i18n),
            Done::Renamed { work, old, new } => self.finish_rename(work, &old, &new, notify, i18n),
        }
    }
}
