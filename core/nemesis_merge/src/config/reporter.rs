use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Status, config::StatusReporterFn};

/// Status Report
///
/// # Throttling
/// `increment` may be called once per patch (tens of thousands of times) from parallel loops,
/// and each report is costly on the GUI side (e.g. Tauri IPC `emit`, egui repaint).
/// Therefore, a report is sent only when the percentage advances, plus the first and the last one.
pub(crate) struct StatusReportCounter<'a> {
    status_reporter: &'a StatusReporterFn,
    to_status: fn(usize, usize) -> Status,
    total: usize,
    /// 0 based index
    counter: AtomicUsize,
    /// Last reported percentage.
    last_percent: AtomicUsize,
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

impl ReportType {
    /// Returns the constructor of [`Status`] as `(index, total) -> Status`.
    const fn to_status_fn(&self) -> fn(usize, usize) -> Status {
        match self {
            Self::GeneratingFnisPatches => {
                |index, total| Status::GeneratingFnisPatches { index, total }
            }
            Self::ReadingPatches => |index, total| Status::ReadingPatches { index, total },
            Self::ParsingPatches => |index, total| Status::ParsingPatches { index, total },
            Self::ApplyingPatches => |index, total| Status::ApplyingPatches { index, total },
            Self::GeneratingHkxFiles => |index, total| Status::GeneratingHkxFiles { index, total },
        }
    }
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
        let to_status = kind.to_status_fn();
        if let Some(status_reporter) = status_reporter {
            (status_reporter)(to_status(0, total));
        }

        Self {
            status_reporter,
            to_status,
            total,
            counter: AtomicUsize::new(0),
            last_percent: AtomicUsize::new(0),
        }
    }

    /// index += 1
    ///
    /// Reports only when the percentage advances or when the last item is done.
    ///
    /// # Note
    /// Reports may arrive out of order between threads, so the receiver should display `max(index)`.
    #[inline]
    pub(crate) fn increment(&self) {
        let done = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        let Some(status_reporter) = self.status_reporter else {
            return;
        };

        if self.should_report(done) {
            (status_reporter)((self.to_status)(done, self.total));
        }
    }

    /// Returns `true` if `done` is the first call that reaches a new percentage, or the last one.
    #[inline]
    fn should_report(&self, done: usize) -> bool {
        let percent = done * 100 / self.total.max(1);
        let prev = self.last_percent.fetch_max(percent, Ordering::Relaxed);
        percent > prev || done == self.total
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rayon::prelude::*;

    use super::*;

    fn collecting_reporter() -> (StatusReporterFn, Arc<Mutex<Vec<Status>>>) {
        let reports = Arc::new(Mutex::new(vec![]));
        let sink = Arc::clone(&reports);
        let reporter: StatusReporterFn = Some(Box::new(move |status| {
            sink.lock().unwrap_or_else(|e| e.into_inner()).push(status);
        }));
        (reporter, reports)
    }

    /// Concurrent increments: each percentage is reported at most once, and the last value is always reported.
    #[test]
    fn concurrent_increment_is_throttled() {
        const TOTAL: usize = 100_000;
        let (reporter, reports) = collecting_reporter();

        {
            let counter = StatusReportCounter::new(&reporter, ReportType::ApplyingPatches, TOTAL);
            (0..TOTAL).into_par_iter().for_each(|_| counter.increment());
            assert_eq!(counter.counter.load(Ordering::Relaxed), TOTAL);
        }

        let reports = core::mem::take(&mut *reports.lock().unwrap_or_else(|e| e.into_inner()));
        let indexes: Vec<usize> = reports
            .iter()
            .map(|status| match status {
                Status::ApplyingPatches { index, total } => {
                    assert_eq!(*total, TOTAL);
                    *index
                }
                other => panic!("unexpected status: {other:?}"),
            })
            .collect();

        // At most: initial(0) + one per percentage(1..=100) + one duplicated last report.
        // (Under contention a percentage may be skipped, so only the upper bound is fixed.)
        assert!(indexes.len() <= 102, "too many reports: {}", indexes.len());
        assert_eq!(indexes.first(), Some(&0));
        assert_eq!(indexes.iter().max(), Some(&TOTAL));
    }

    #[test]
    fn small_total_reports_every_step() {
        let (reporter, reports) = collecting_reporter();
        let counter = StatusReportCounter::new(&reporter, ReportType::ReadingPatches, 3);
        counter.increment();
        counter.increment();
        counter.increment();

        let reports = core::mem::take(&mut *reports.lock().unwrap_or_else(|e| e.into_inner()));
        assert_eq!(
            reports,
            vec![
                Status::ReadingPatches { index: 0, total: 3 },
                Status::ReadingPatches { index: 1, total: 3 },
                Status::ReadingPatches { index: 2, total: 3 },
                Status::ReadingPatches { index: 3, total: 3 },
            ]
        );
    }
}
