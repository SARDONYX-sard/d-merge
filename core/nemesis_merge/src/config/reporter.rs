use crate::{config::StatusReporterFn, Status};

/// Status Report
pub(crate) struct StatusReportCounter<'a> {
    status_reporter: &'a StatusReporterFn,
    kind: ReportType,
    total: usize,
    /// 0 based index
    counter: std::sync::atomic::AtomicUsize,
}

pub(crate) enum ReportType {
    /// Status when generating FNIS patches.
    GeneratingFnisPatches,

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
    /// NOTE: The 0th one also serves as the initial report execution.
    /// This is to avoid the appearance of freezing after loading the FNIS mod.
    #[inline]
    pub(crate) fn new(
        status_reporter: &'a StatusReporterFn,
        kind: ReportType,
        total: usize,
    ) -> Self {
        if let Some(status_reporter) = status_reporter {
            match kind {
                ReportType::GeneratingFnisPatches => {
                    (status_reporter)(Status::GeneratingFnisPatches { index: 0, total });
                }
                ReportType::ReadingPatches => {
                    (status_reporter)(Status::ReadingPatches { index: 0, total });
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

        Self {
            status_reporter,
            total,
            counter: std::sync::atomic::AtomicUsize::new(0),
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
                ReportType::GeneratingFnisPatches => {
                    (status_reporter)(Status::GeneratingFnisPatches {
                        index: done,
                        total: self.total,
                    });
                }
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
