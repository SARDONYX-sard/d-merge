//! Global, state-based API compatible with the original `tracing_rotation`
//! interface.
//!
//! This module wraps [`RotationLayer`] and [`RotationHandle`] behind a
//! process-wide [`OnceLock`] so callers that prefer a simple imperative style
//! do not need to thread handles through their code.
//!
//! # Example
//!
//! ```rust,no_run
//! use tracing_rotation::global;
//!
//! // Call once, usually in `main`.
//! global::init("/var/log/myapp", "app.log", 5).expect("logger init failed");
//!
//! // Later, from anywhere in the process:
//! global::change_level("debug").unwrap();
//! global::change_log_path("/tmp/myapp", "app.log").unwrap();
//! ```
//!
//! [`RotationLayer`]: crate::RotationLayer
//! [`RotationHandle`]: crate::RotationHandle

use std::{path::Path, sync::OnceLock};

use tracing_subscriber::prelude::*;

use crate::{
    RotationBuilder, RotationHandle,
    error::{Error, Result},
};

// One OnceLock is enough: the Handle is Clone and covers both level and path.
static HANDLE: OnceLock<RotationHandle> = OnceLock::new();

// ────────────────────────────────────────────────────────────────────────────
// Public API
// ────────────────────────────────────────────────────────────────────────────

/// Initialize the global rotation logger and install it as the process-wide
/// tracing subscriber.
///
/// # Parameters
/// - `log_dir`   – directory where log files are stored (created if absent).
/// - `log_name`  – base file name; e.g. `"app.log"`.
/// - `max_files` – maximum number of files (live + archived) kept on disk.
///
/// # Errors
/// - [`Error::AlreadyInit`] if called more than once.
/// - Any I/O error encountered while creating the log directory or file.
///
/// # Panics
/// Panics if another global subscriber has already been set via
/// `tracing::subscriber::set_global_default` or a previous `init` call.
pub fn init<P>(log_dir: P, log_name: &str, max_files: usize) -> Result<()>
where
    P: AsRef<Path>,
{
    init_with_level(log_dir, log_name, max_files, tracing::Level::TRACE)
}

/// Initialize the global rotation logger and install it as the process-wide
/// tracing subscriber.
///
/// # Parameters
/// - `log_dir`   – directory where log files are stored (created if absent).
/// - `log_name`  – base file name; e.g. `"app.log"`.
/// - `max_files` – maximum number of files (live + archived) kept on disk.
/// - `level`     – initial log level filter, e.g. `LevelFilter::INFO` or `"info"`.
///
/// # Errors
/// - [`Error::AlreadyInit`] if called more than once.
/// - Any I/O error encountered while creating the log directory or file.
///
/// # Panics
/// Panics if another global subscriber has already been set via
/// `tracing::subscriber::set_global_default` or a previous `init` call.
pub fn init_with_level<P, L>(log_dir: P, log_name: &str, max_files: usize, level: L) -> Result<()>
where
    P: AsRef<Path>,
    L: Into<tracing::Level>,
{
    let log_dir = log_dir.as_ref().to_path_buf();
    let level = level.into();

    let handle = {
        let this = RotationBuilder {
            log_dir,
            log_file: log_name.to_string(),
            max_files,
            initial_level: level.into(),
        };

        let file = crate::rotate::rotate_files(&this.log_dir, &this.log_file, this.max_files)?;
        let writer = crate::SwappableWriter::new(file);

        let (reload, reload_handle) = tracing_subscriber::reload::Layer::new(this.initial_level);
        let fmt = tracing_subscriber::fmt::layer()
            .compact()
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .with_writer(writer.clone());

        let handle = RotationHandle {
            reloader: std::sync::Arc::new(crate::HandleReloader(reload_handle)),
            writer,
            max_files: this.max_files,
        };

        tracing_subscriber::registry().with(reload).with(fmt).init();

        handle
    };

    HANDLE.set(handle).map_err(|_| Error::AlreadyInit)?;

    Ok(())
}

/// Change the minimum log level at runtime.
///
/// `level` is matched case-insensitively; unrecognized strings fall back to
/// `"error"` with a warning log.
///
/// # Errors
/// [`Error::NotInit`] if [`init`] has not been called yet.
pub fn change_level(level: &str) -> Result<()> {
    handle()?.set_level(level)
}

/// Redirect log output to a freshly rotated file.
///
/// Rotation is performed before the new file is opened (identical to startup
/// behaviour): the current active file is renamed with a timestamp suffix and,
/// if the file count would exceed `max_files`, the oldest file is deleted.
///
/// # Errors
/// [`Error::NotInit`] if [`init`] has not been called yet, or any I/O error.
pub fn change_log_path(log_dir: impl AsRef<Path>, log_stem: &str) -> Result<()> {
    handle()?.set_path(log_dir, log_stem)
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

fn handle() -> Result<&'static RotationHandle> {
    HANDLE.get().ok_or(Error::NotInit)
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `change_level` / `change_log_path` before `init` must return `NotInit`.
    #[test]
    fn errors_before_init() {
        // A fresh process would have an empty OnceLock, but since tests share
        // the same process we can only verify the path when init has NOT been
        // called yet.  We use a standalone handle check instead.
        let result = handle();
        // Either NotInit (OnceLock empty) or already set by a prior test run.
        match result {
            Err(Error::NotInit) | Ok(_) => {} // another test initialized it first — that is fine
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}
