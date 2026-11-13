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
//! # Why a `std::thread` (not rayon or tokio)
//! - The heavy part is parallel already, with rayon: `load_counted` reads the behaviors with
//!   `par_iter`, `rename_code` their objects. The job's thread only takes it off the UI thread and is
//!   waited for.
//! - `rayon::spawn` returns no `JoinHandle` (a channel would be needed for the result), and the job
//!   would hold one worker of the pool until it ends: one thread less for its own `par_iter`.
//! - tokio is for async IO; this is CPU and blocking file reads, so it would be `spawn_blocking` (a
//!   thread too), plus a runtime handle for this crate to get.
//! - Only one job runs at a time (`PatchTool::job` is an `Option`): a pool brings nothing.
//!
//! Worth changing if jobs are to be cancelled or to run several at once.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::JoinHandle,
};

use egui_widgets::notify::Notification;
use gui_shared::i18n::{I18nKey, I18nMap};

use crate::patch::{PatchTool, workspace::Workspace};

/// What a reload keeps of the work: it is not in the mod. See `PatchTool::reload_from_mod`.
#[derive(Debug)]
pub(in crate::patch) struct Kept {
    pub(in crate::patch) output: String,
    pub(in crate::patch) refs: Vec<String>,
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

/// A job running. See the module docs.
#[derive(Debug)]
pub(in crate::patch) struct Job {
    /// What is shown, e.g. `Renaming the mod code`, and after it `detail`, e.g. `oldc -> newc`.
    pub(in crate::patch) label: I18nKey,
    pub(in crate::patch) detail: String,
    /// What it went through so far.
    pub(in crate::patch) done: Arc<AtomicUsize>,
    /// Of how many: `0` if that is not known beforehand.
    pub(in crate::patch) total: usize,
    work: JoinHandle<Done>,
}

impl Job {
    /// Runs `run` on another thread. It counts in the `AtomicUsize` it is given.
    pub(in crate::patch) fn spawn(
        label: I18nKey,
        detail: String,
        total: usize,
        run: impl FnOnce(&AtomicUsize) -> Done + Send + 'static,
    ) -> Self {
        let done = Arc::new(AtomicUsize::new(0));
        let work = {
            let done = Arc::clone(&done);
            std::thread::spawn(move || run(&done))
        };
        Self { label, detail, done, total, work }
    }

    /// How far it is, from 0 to 1. `None`: the total is not known.
    pub(in crate::patch) fn progress(&self) -> Option<f32> {
        #[expect(clippy::cast_precision_loss, reason = "a ratio for a progress bar")]
        let ratio = (self.total > 0).then(|| self.done.load(Ordering::Relaxed) as f32 / self.total as f32);
        ratio.map(|ratio| ratio.min(1.0))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_counts_and_ends() {
        let job = Job::spawn(I18nKey::NemesisReadingWork, String::new(), 4, |done| {
            done.fetch_add(2, Ordering::Relaxed);
            Done::Loaded { loaded: Ok(Workspace::default()), code: "code".to_owned() }
        });
        while !job.work.is_finished() {
            std::thread::yield_now();
        }
        assert_eq!(job.progress(), Some(0.5));
        let unknown = Job { total: 0, ..job };
        assert_eq!(unknown.progress(), None);
    }
}
