//! How long reading the Nemesis patch files of each mod took: it is the slowest step of a run on a cold
//! OS file cache, and this tells which mods it is for.
//!
//! # Example of the log
//! ```text
//! DEBUG Slowest mods to read rank=1 mod_dir="D:/mods/BFCO/Nemesis_Engine/mod/bfco" files=4213 kib=9120 read=2.41s elapsed=812ms
//! ```
//!
//! - `read`: the time in the reads of its files, added up over the threads. What ranks the mods: it
//!   only has the time of this mod.
//! - `elapsed`: from the start to the end of the mod, with its walk of the directories. On a thread
//!   waiting for its other parts, rayon runs parts of other mods, which this also has.
//!
//! Only measured while the log would show it, see [`is_enabled`]: a run has tens of thousands of files.

use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// How many mods the log has.
const SLOWEST_MODS: usize = 10;

/// Whether the debug log is on, which is all that the measuring is for.
#[inline]
pub(super) fn is_enabled() -> bool {
    #[cfg(feature = "tracing")]
    return tracing::enabled!(tracing::Level::DEBUG);
    #[cfg(not(feature = "tracing"))]
    return false;
}

/// Reads a file, measured if there is a `timer`.
#[inline]
pub(super) fn read_to_string(timer: Option<&ModTimer<'_>>, path: &Path) -> std::io::Result<String> {
    match timer {
        Some(timer) => timer.read_to_string(path),
        None => std::fs::read_to_string(path),
    }
}

/// Measures one mod while its files are read in parallel.
#[derive(Debug)]
pub(super) struct ModTimer<'a> {
    mod_dir: &'a str,
    start: Instant,
    files: AtomicU64,
    bytes: AtomicU64,
    read_nanos: AtomicU64,
}

impl<'a> ModTimer<'a> {
    pub(super) fn start(mod_dir: &'a str) -> Self {
        Self {
            mod_dir,
            start: Instant::now(),
            files: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            read_nanos: AtomicU64::new(0),
        }
    }

    /// [`std::fs::read_to_string`], measured.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let start = Instant::now();
        let content = std::fs::read_to_string(path);
        let nanos = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);

        self.read_nanos.fetch_add(nanos, Ordering::Relaxed);
        self.files.fetch_add(1, Ordering::Relaxed);
        if let Ok(content) = &content {
            self.bytes.fetch_add(content.len() as u64, Ordering::Relaxed);
        }
        content
    }

    pub(super) fn finish(self) -> ModReadTime<'a> {
        ModReadTime {
            mod_dir: self.mod_dir,
            files: self.files.into_inner(),
            bytes: self.bytes.into_inner(),
            read: Duration::from_nanos(self.read_nanos.into_inner()),
            elapsed: self.start.elapsed(),
        }
    }
}

/// See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ModReadTime<'a> {
    pub(super) mod_dir: &'a str,
    pub(super) files: u64,
    pub(super) bytes: u64,
    pub(super) read: Duration,
    pub(super) elapsed: Duration,
}

/// The `count` mods with the longest reads, the longest first.
fn slowest<'a>(mut mods: Vec<ModReadTime<'a>>, count: usize) -> Vec<ModReadTime<'a>> {
    mods.sort_unstable_by(|a, b| b.read.cmp(&a.read).then_with(|| a.mod_dir.cmp(b.mod_dir)));
    mods.truncate(count);
    mods
}

/// Writes the slowest mods to the debug log.
pub(super) fn log_slowest_mods(mods: Vec<ModReadTime<'_>>) {
    #[cfg(feature = "tracing")]
    for (rank, time) in slowest(mods, SLOWEST_MODS).into_iter().enumerate() {
        tracing::debug!(
            rank = rank + 1,
            mod_dir = time.mod_dir,
            files = time.files,
            kib = time.bytes / 1024,
            read = ?time.read,
            elapsed = ?time.elapsed,
            "Slowest mods to read"
        );
    }
    #[cfg(not(feature = "tracing"))]
    drop(mods);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(mod_dir: &str, read_ms: u64) -> ModReadTime<'_> {
        ModReadTime { mod_dir, files: 1, bytes: 1, read: Duration::from_millis(read_ms), elapsed: Duration::ZERO }
    }

    #[test]
    fn the_longest_reads_come_first_and_only_as_many_as_asked() {
        let mods = vec![time("a", 10), time("b", 30), time("c", 20), time("d", 5)];
        let dirs: Vec<_> = slowest(mods, 2).into_iter().map(|time| time.mod_dir).collect();
        assert_eq!(dirs, ["b", "c"]);
    }

    #[test]
    fn without_a_timer_a_file_is_just_read() {
        let path = std::env::temp_dir().join(format!("d_merge_read_stats_plain_{}.txt", std::process::id()));
        std::fs::write(&path, "12345").unwrap();
        let content = read_to_string(None, &path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(content.unwrap(), "12345");
    }

    #[test]
    fn a_mod_counts_its_files_also_those_it_could_not_read() {
        let dir = std::env::temp_dir().join(format!("d_merge_read_stats_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("#0001.txt");
        std::fs::write(&file, "12345").unwrap();

        let timer = ModTimer::start("mod");
        assert_eq!(timer.read_to_string(&file).unwrap(), "12345");
        assert!(timer.read_to_string(&dir.join("missing.txt")).is_err());
        let time = timer.finish();

        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!((time.mod_dir, time.files, time.bytes), ("mod", 2, 5));
        assert!(time.elapsed >= time.read);
    }
}
