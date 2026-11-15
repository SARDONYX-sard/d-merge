//! How long reading the Nemesis patch files of each mod took: it is the slowest step of a run on a cold
//! OS file cache, and this tells which mods it is for.
//!
//! # Example of the log
//! Every mod, the longest first:
//! ```text
//! DEBUG Read mod rank=1 secs=12.10 mod_dir="D:/mods/CGO/Nemesis_Engine/mod/dscgo" files=3219 kib=3681 per_file=10.29ms threads=2.7
//! ```
//!
//! - `secs`: how long its files were read, as a clock sees it: from the start of its first read to the
//!   end of its last one. What ranks the mods.
//!   (Not from the start of the mod to its end: a thread waiting for its other parts runs parts of
//!   other mods meanwhile (rayon), and that end would have them too.)
//! - `per_file`: the time to read one file, on average. ~0.05 ms on a warm OS file cache; ~1-10 ms
//!   on a cold one, which is most of a run.
//! - `threads`: how many threads read its files at once, on average (the time in its reads, added up
//!   over the threads, / `secs`).
//!
//! Only measured while the log would show it, see [`is_enabled`]: a run has tens of thousands of files.

use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

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
    /// What the times below are counted from (nanoseconds).
    origin: Instant,
    files: AtomicU64,
    bytes: AtomicU64,
    /// The time in the reads, added up over the threads.
    read_nanos: AtomicU64,
    /// The start of the first read. `u64::MAX`: none yet.
    first_start: AtomicU64,
    /// The end of the last read.
    last_end: AtomicU64,
}

impl<'a> ModTimer<'a> {
    pub(super) fn start(mod_dir: &'a str) -> Self {
        Self {
            mod_dir,
            origin: Instant::now(),
            files: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            read_nanos: AtomicU64::new(0),
            first_start: AtomicU64::new(u64::MAX),
            last_end: AtomicU64::new(0),
        }
    }

    /// Nanoseconds since `origin`.
    fn now(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    /// [`std::fs::read_to_string`], measured.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let start = self.now();
        let content = std::fs::read_to_string(path);
        let end = self.now();

        self.first_start.fetch_min(start, Ordering::Relaxed);
        self.last_end.fetch_max(end, Ordering::Relaxed);
        self.read_nanos.fetch_add(end - start, Ordering::Relaxed);
        self.files.fetch_add(1, Ordering::Relaxed);
        if let Ok(content) = &content {
            self.bytes.fetch_add(content.len() as u64, Ordering::Relaxed);
        }
        content
    }

    pub(super) const fn finish(self) -> ModReadTime<'a> {
        let (first_start, last_end) = (self.first_start.into_inner(), self.last_end.into_inner());
        ModReadTime {
            mod_dir: self.mod_dir,
            files: self.files.into_inner(),
            bytes: self.bytes.into_inner(),
            reading: Duration::from_nanos(last_end.saturating_sub(first_start)),
            read_total: Duration::from_nanos(self.read_nanos.into_inner()),
        }
    }
}

/// See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ModReadTime<'a> {
    pub(super) mod_dir: &'a str,
    pub(super) files: u64,
    pub(super) bytes: u64,
    /// From the start of its first read to the end of its last one: the log's `secs`.
    pub(super) reading: Duration,
    /// The time in its reads, added up over the threads: longer than `reading` when they overlap.
    pub(super) read_total: Duration,
}

impl ModReadTime<'_> {
    /// The time to read one file, on average.
    fn per_file(&self) -> Duration {
        match u32::try_from(self.files) {
            Ok(files) if files > 0 => self.read_total / files,
            _ => Duration::ZERO,
        }
    }

    /// How many threads read its files at once, on average.
    fn threads(&self) -> f64 {
        if self.reading.is_zero() { 0.0 } else { self.read_total.as_secs_f64() / self.reading.as_secs_f64() }
    }
}

/// The mods, the longest `reading` first.
fn longest_first(mut mods: Vec<ModReadTime<'_>>) -> Vec<ModReadTime<'_>> {
    mods.sort_unstable_by(|a, b| b.reading.cmp(&a.reading).then_with(|| a.mod_dir.cmp(b.mod_dir)));
    mods
}

/// Writes how long each mod took to the debug log, the longest first.
pub(super) fn log_mod_read_times(mods: Vec<ModReadTime<'_>>) {
    #[cfg(feature = "tracing")]
    for (rank, time) in longest_first(mods).into_iter().enumerate() {
        tracing::debug!(
            rank = rank + 1,
            secs = format_args!("{:.2}", time.reading.as_secs_f64()),
            mod_dir = time.mod_dir,
            files = time.files,
            kib = time.bytes / 1024,
            per_file = ?time.per_file(),
            threads = format_args!("{:.1}", time.threads()),
            "Read mod"
        );
    }
    #[cfg(not(feature = "tracing"))]
    drop(mods);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(mod_dir: &str, reading_ms: u64) -> ModReadTime<'_> {
        let reading = Duration::from_millis(reading_ms);
        ModReadTime { mod_dir, files: 1, bytes: 1, reading, read_total: reading }
    }

    #[test]
    fn every_mod_is_logged_the_longest_first() {
        let mods = vec![time("a", 10), time("b", 30), time("c", 20), time("d", 5)];
        let dirs: Vec<_> = longest_first(mods).into_iter().map(|time| time.mod_dir).collect();
        assert_eq!(dirs, ["b", "c", "a", "d"]);
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
        // One thread, one read after the other: the reads fill the time.
        assert!(time.reading >= time.read_total);
    }

    #[test]
    fn a_mod_without_files_took_no_time() {
        let time = ModTimer::start("empty").finish();
        assert_eq!((time.reading, time.per_file(), time.threads()), (Duration::ZERO, Duration::ZERO, 0.0));
    }

    /// The log's example: 3219 files read over 12.10 s, 33.12 s of reads added up.
    #[test]
    fn per_file_and_threads_are_of_the_reads() {
        let time = ModReadTime {
            mod_dir: "dscgo",
            files: 3219,
            bytes: 0,
            reading: Duration::from_millis(12_100),
            read_total: Duration::from_millis(33_120),
        };
        assert_eq!(time.per_file().as_micros(), 10_288);
        assert!((time.threads() - 2.74).abs() < 0.01, "{}", time.threads());
    }
}
