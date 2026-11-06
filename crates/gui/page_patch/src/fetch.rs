//! Reads the mod list on a worker thread, so a slow disk does not freeze the UI.
//!
//! ```text
//! start()   Idle -> Fetching                 (UI thread)
//! worker    Fetching -> Done / Empty / Error (worker thread, results in `fetched`)
//! poll()    Done / Empty / Error -> Idle     (UI thread, returns the result once)
//! ```

use std::{sync::Arc, time::Duration};

use gui_shared::fetch::FetchState;
use mod_info::ModInfo;
use parking_lot::RwLock;

/// What happened to the fetch since the last [`ModListFetch::poll`].
#[derive(Debug)]
pub(crate) enum FetchEvent {
    /// Still reading.
    Fetching,
    Fetched {
        mods: Vec<ModInfo>,
        elapsed: Duration,
    },
    /// Read fine, but there are no mods.
    Empty {
        elapsed: Duration,
    },
    /// See the log for why.
    Failed {
        elapsed: Duration,
    },
}

/// See the module docs.
#[derive(Debug, Default)]
pub(crate) struct ModListFetch {
    state: Arc<RwLock<FetchState>>,
    /// Filled by the worker before it switches to `Done`.
    fetched: Arc<RwLock<Vec<ModInfo>>>,
    /// The last fetch found no mods: the table shows nothing, but the stored check states stay.
    pub(crate) last_was_empty: bool,
}

impl ModListFetch {
    pub(crate) fn is_fetching(&self) -> bool {
        matches!(*self.state.read(), FetchState::Fetching)
    }

    /// Reads the list of `dir` in the background. An earlier fetch still running is superseded:
    /// both write to the same place, so the last result wins.
    pub(crate) fn start(&self, dir: String, use_vfs: bool) {
        tracing::debug!("fetching the mod list of `{dir}`");
        *self.state.write() = FetchState::Fetching;
        let state = Arc::clone(&self.state);
        let fetched = Arc::clone(&self.fetched);
        let start_time = std::time::Instant::now();

        std::thread::spawn(move || {
            // NOTE: If rayon saturates all CPU threads the UI freezes.
            // `mod_info::get_all` must not spawn unbounded rayon work internally.
            let new_state = match mod_info::get_all(&dir, use_vfs) {
                Ok(mods) if mods.is_empty() => FetchState::Empty { elapsed: start_time.elapsed() },
                Ok(mods) => {
                    *fetched.write() = mods;
                    FetchState::Done { elapsed: start_time.elapsed() }
                }
                Err(e) => {
                    tracing::error!(%e, "mod_info::get_all error");
                    FetchState::Error { elapsed: start_time.elapsed() }
                }
            };
            *state.write() = new_state;
        });
    }

    /// The result, once. `try_read`: it never blocks the UI thread while the worker writes.
    pub(crate) fn poll(&mut self) -> Option<FetchEvent> {
        let state = *self.state.try_read()?;
        let event = match state {
            FetchState::Idle => return None,
            FetchState::Fetching => return Some(FetchEvent::Fetching),
            FetchState::Done { elapsed } => {
                self.last_was_empty = false;
                FetchEvent::Fetched { mods: std::mem::take(&mut *self.fetched.write()), elapsed }
            }
            FetchState::Empty { elapsed } => {
                self.last_was_empty = true;
                FetchEvent::Empty { elapsed }
            }
            FetchState::Error { elapsed } => FetchEvent::Failed { elapsed },
        };
        *self.state.write() = FetchState::Idle;
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(state: FetchState) -> ModListFetch {
        let fetch = ModListFetch::default();
        *fetch.state.write() = state;
        fetch
    }

    #[test]
    fn a_result_is_reported_once() {
        let mut fetch = finished(FetchState::Done { elapsed: Duration::ZERO });
        assert!(matches!(fetch.poll(), Some(FetchEvent::Fetched { .. })));
        assert!(fetch.poll().is_none());
    }

    #[test]
    fn remembers_an_empty_result_until_the_next_success() {
        let mut fetch = finished(FetchState::Empty { elapsed: Duration::ZERO });
        assert!(matches!(fetch.poll(), Some(FetchEvent::Empty { .. })));
        assert!(fetch.last_was_empty);

        *fetch.state.write() = FetchState::Done { elapsed: Duration::ZERO };
        fetch.poll();
        assert!(!fetch.last_was_empty);
    }

    #[test]
    fn fetching_is_reported_until_it_ends() {
        let mut fetch = finished(FetchState::Fetching);
        assert!(fetch.is_fetching());
        assert!(matches!(fetch.poll(), Some(FetchEvent::Fetching)));
        assert!(fetch.is_fetching());
    }
}
