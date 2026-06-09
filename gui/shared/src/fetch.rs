/// Represents the lifecycle of a background mod-list fetch.
///
/// The background thread writes one of the terminal variants ([`Done`],
/// [`Empty`], [`Error`]) when it finishes; the UI thread polls every frame
/// and transitions back to [`Idle`] after consuming the result.
///
/// ```text
/// Idle ──start──▶ Fetching ──ok / non-empty──▶ Done
///                     │
///                     ├──ok / empty──────────▶ Empty
///                     └──error───────────────▶ Error
/// Done | Empty | Error ──consumed──▶ Idle
/// ```
///
/// [`Done`]: FetchState::Done
/// [`Empty`]: FetchState::Empty
/// [`Error`]: FetchState::Error
/// [`Idle`]: FetchState::Idle
#[derive(Debug)]
pub enum FetchState {
    /// No fetch is in progress; the mod list is up-to-date (or never loaded).
    Idle,

    /// A background worker thread is currently fetching mod info.
    Fetching,

    /// The fetch succeeded and returned at least one item.
    ///
    /// `elapsed` is the wall-clock duration of the worker call, displayed in
    /// the status bar as `"Done (0.42 s)"`.
    Done { elapsed: std::time::Duration },

    /// The fetch succeeded but the directory contained zero mod entries.
    ///
    /// The UI preserves the existing check-state rather than clearing the
    /// list, so the user does not lose their selections on an empty scan.
    Empty { elapsed: std::time::Duration },

    /// The fetch failed (I/O error, invalid path, etc.).
    ///
    /// The error is logged via `tracing::error!` inside the worker; the UI
    /// shows a red status message using the elapsed time.
    Error { elapsed: std::time::Duration },
}
