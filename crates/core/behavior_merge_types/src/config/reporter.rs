use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::{Status, StatusReporterFn};

/// Status Report
pub struct StatusReportCounter<'a> {
    status_reporter: &'a StatusReporterFn,
    kind: ReportType,
    total: usize,
    /// 0 based index
    counter: std::sync::atomic::AtomicUsize,
}

#[derive(Debug, Clone, Copy)]
pub enum ReportType {
    /// Status when generating FNIS patches.
    GeneratingFnisPatches,

    /// Status when Parsing patches.
    ParsingPatches,

    /// Status when applying patches.
    ApplyingPatches,

    /// Status when generating HKX files.
    GeneratingHkxFiles,
}

impl<'a> StatusReportCounter<'a> {
    /// NOTE: The 0th one also serves as the initial report execution.
    /// This is to avoid the appearance of freezing after loading the FNIS mod.
    #[inline]
    pub fn new(status_reporter: &'a StatusReporterFn, kind: ReportType, total: usize) -> Self {
        if let Some(status_reporter) = status_reporter {
            match kind {
                ReportType::GeneratingFnisPatches => {
                    (status_reporter)(Status::GeneratingFnisPatches { index: 0, total });
                }
                ReportType::ParsingPatches => {
                    (status_reporter)(Status::ParsingPatches { index: 0, total });
                }
                ReportType::ApplyingPatches => {
                    (status_reporter)(Status::ApplyingPatches { index: 0, total });
                }
                ReportType::GeneratingHkxFiles => {
                    (status_reporter)(Status::GeneratingHkxFiles { index: 0, total });
                }
            };
        }

        Self { status_reporter, total, counter: std::sync::atomic::AtomicUsize::new(0), kind }
    }

    /// index += 1
    #[inline]
    pub fn increment(&self) {
        let done = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        if let Some(status_reporter) = self.status_reporter {
            match self.kind {
                ReportType::GeneratingFnisPatches => {
                    (status_reporter)(Status::GeneratingFnisPatches { index: done, total: self.total });
                }
                ReportType::ParsingPatches => {
                    (status_reporter)(Status::ParsingPatches { index: done, total: self.total });
                }
                ReportType::ApplyingPatches => {
                    (status_reporter)(Status::ApplyingPatches { index: done, total: self.total });
                }
                ReportType::GeneratingHkxFiles => {
                    (status_reporter)(Status::GeneratingHkxFiles { index: done, total: self.total });
                }
            };
        }
    }
}

impl core::fmt::Debug for StatusReportCounter<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The reporter callback itself is not `Debug`.
        f.debug_struct("StatusReportCounter")
            .field("kind", &self.kind)
            .field("total", &self.total)
            .field("counter", &self.counter)
            .finish_non_exhaustive()
    }
}

/// Reports [`Status::ReadingPatches`]: the mod dirs read, and the files read in them so far.
///
/// ```text
/// new(69)       -> (0/69 mods, 0 files)
/// file_read()   -> (0/69 mods, 1 files)     each file, from any thread
/// mod_read()    -> (1/69 mods, 3219 files)  each mod dir, once all its files are read
/// ```
pub struct ReadingReporter<'a> {
    status_reporter: &'a StatusReporterFn,
    /// Number of mod dirs.
    total: usize,
    mods: AtomicUsize,
    files: AtomicUsize,
}

impl<'a> ReadingReporter<'a> {
    /// Reports the start (nothing read yet) at once: the reading can take seconds before the first
    /// file is done.
    #[inline]
    pub fn new(status_reporter: &'a StatusReporterFn, total: usize) -> Self {
        let reporter = Self { status_reporter, total, mods: AtomicUsize::new(0), files: AtomicUsize::new(0) };
        reporter.report(0, 0);
        reporter
    }

    /// One more file is read.
    #[inline]
    pub fn file_read(&self) {
        let files = self.files.fetch_add(1, Ordering::Relaxed) + 1;
        self.report(self.mods.load(Ordering::Relaxed), files);
    }

    /// One more mod dir is read, all its files.
    #[inline]
    pub fn mod_read(&self) {
        let mods = self.mods.fetch_add(1, Ordering::Relaxed) + 1;
        self.report(mods, self.files.load(Ordering::Relaxed));
    }

    fn report(&self, index: usize, files: usize) {
        if let Some(status_reporter) = self.status_reporter {
            (status_reporter)(Status::ReadingPatches { index, total: self.total, files });
        }
    }
}

impl core::fmt::Debug for ReadingReporter<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The reporter callback itself is not `Debug`.
        f.debug_struct("ReadingReporter")
            .field("total", &self.total)
            .field("mods", &self.mods)
            .field("files", &self.files)
            .finish_non_exhaustive()
    }
}
