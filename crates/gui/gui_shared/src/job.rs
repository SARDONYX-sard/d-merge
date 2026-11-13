//! Work too long for a frame on another thread, while the page shows how far it is.
//!
//! ```text
//! let job = Job::spawn(total, |done| { for file in files { read(file); done.fetch_add(1, Relaxed); } made });
//! each frame:  job.progress() -> Some(0.4)        (`None` while the total is not known)
//!              job.take_if_finished() -> Some(Ok(made)) once, then the job is dropped
//! ```
//! A plain `std::thread`: the heavy part is parallel inside (rayon) already, the thread only takes it
//! off the UI thread; `rayon::spawn` gives no handle for the result, and tokio is for async IO.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread::JoinHandle,
};

/// See the module docs. `T`: what the work makes.
#[derive(Debug)]
pub struct Job<T> {
    /// What it went through so far, counted by the work.
    done: Arc<AtomicUsize>,
    /// Of how many: `0` if that is not known beforehand.
    total: usize,
    work: JoinHandle<T>,
}

impl<T: Send + 'static> Job<T> {
    /// Runs `run` on another thread; it counts in the `AtomicUsize` it is given, out of `total`.
    pub fn spawn(total: usize, run: impl FnOnce(&AtomicUsize) -> T + Send + 'static) -> Self {
        let done = Arc::new(AtomicUsize::new(0));
        let work = {
            let done = Arc::clone(&done);
            std::thread::spawn(move || run(&done))
        };
        Self { done, total, work }
    }
}

impl<T> Job<T> {
    /// `(done, total)` so far, e.g. `(120, 300)`; `total` is `0` if it is not known.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        (self.done.load(Ordering::Relaxed), self.total)
    }

    /// How far it is, from 0 to 1. `None`: the total is not known.
    #[must_use]
    pub fn progress(&self) -> Option<f32> {
        let (done, total) = self.counts();
        #[expect(clippy::cast_precision_loss, reason = "a ratio for a progress bar")]
        let ratio = (total > 0).then(|| done as f32 / total as f32);
        ratio.map(|ratio| ratio.min(1.0))
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.work.is_finished()
    }

    /// Waits for the work: what it made, or `Err` if it panicked.
    ///
    /// # Errors
    /// The work panicked.
    pub fn join(self) -> std::thread::Result<T> {
        self.work.join()
    }

    /// Takes the job out of `slot` once it is finished: what it made, or `Err` if it panicked.
    /// `None`: none, or still running.
    pub fn take_if_finished(slot: &mut Option<Self>) -> Option<std::thread::Result<T>> {
        slot.take_if(|job| job.is_finished()).map(Self::join)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_counts_and_ends() {
        let mut slot = Some(Job::spawn(4, |done| {
            done.fetch_add(2, Ordering::Relaxed);
            "made"
        }));
        while !slot.as_ref().is_some_and(Job::is_finished) {
            std::thread::yield_now();
        }
        assert_eq!(slot.as_ref().map(Job::progress), Some(Some(0.5)));
        assert_eq!(Job::take_if_finished(&mut slot).map(Result::unwrap), Some("made"));
        assert!(slot.is_none());

        let unknown = Job::spawn(0, |_| ());
        assert_eq!(unknown.progress(), None);
    }
}
