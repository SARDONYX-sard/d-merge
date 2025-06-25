use crate::{config::StatusReporterFn, Status};

/// Status Report
pub(crate) struct StatusReportCounter<'a> {
    status_reporter: &'a StatusReporterFn,
    kind: ReportType,
    total: usize,
    /// 1 based index
    counter: std::sync::atomic::AtomicUsize,
}

pub(crate) enum ReportType {
    /// Status when reading patches.
    ReadingPatches,

    /// Status when Parsing patches.
    ParsingPatches,

    /// Status when applying patches.
    ApplyingPatches,

    /// Status when generating HKX files.
    GeneratingHkxFiles,
}

impl<'a> StatusReportCounter<'a> {
    #[inline]
    pub(crate) fn new(
        status_reporter: &'a StatusReporterFn,
        kind: ReportType,
        total: usize,
    ) -> Self {
        Self {
            status_reporter,
            total,
            counter: std::sync::atomic::AtomicUsize::new(1),
            kind,
        }
    }

    /// index += 1
    #[inline]
    pub(crate) fn increment(&self) {
        let done = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        if let Some(status_reporter) = self.status_reporter {
            match self.kind {
                ReportType::ReadingPatches => (status_reporter)(Status::ReadingPatches {
                    index: done,
                    total: self.total,
                }),
                ReportType::ParsingPatches => (status_reporter)(Status::ParsingPatches {
                    index: done,
                    total: self.total,
                }),
                ReportType::ApplyingPatches => (status_reporter)(Status::ApplyingPatches {
                    index: done,
                    total: self.total,
                }),
                ReportType::GeneratingHkxFiles => {
                    (status_reporter)(Status::GeneratingHkxFiles {
                        index: done,
                        total: self.total,
                    });
                }
            };
        }
    }
}
