//! # tracing-rotation
//!
//! A [`tracing_subscriber::Layer`] that writes to rotating log files, with
//! runtime-adjustable log level and log-file path.
//!
//! ## Global API (drop-in replacement for the original interface)
//!
//! ```rust,no_run
//! use tracing_rotation::global;
//!
//! global::init("/var/log/myapp", "app.log", 5).expect("logger init failed");
//!
//! global::change_level("debug").unwrap();
//! global::change_log_path("/tmp/myapp", "app.log").unwrap();
//! ```

pub mod error;
pub mod global;
pub(crate) mod rotate;

use std::{
    fmt,
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use parking_lot::Mutex;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{fmt as fmt_layer, reload};

use crate::{
    error::{Error, Result},
    rotate::rotate_files,
};

// ────────────────────────────────────────────────────────────────────────────
// SwappableWriter
// ────────────────────────────────────────────────────────────────────────────

/// NOTE: For some reason, the last data written using [`io::BufWriter`] isn't flushed properly.
#[derive(Clone)]
#[expect(missing_debug_implementations)]
pub struct SwappableWriter(Arc<Mutex<File>>);

impl SwappableWriter {
    #[inline]
    fn new(file: File) -> Self {
        Self(Arc::new(Mutex::new(file)))
    }

    #[inline]
    pub(crate) fn swap(&self, file: File) -> Result<()> {
        let mut guard = self.0.lock();
        guard.flush().ok();
        *guard = file;
        drop(guard);
        Ok(())
    }
}

impl io::Write for SwappableWriter {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().flush()
    }
}

impl<'a> fmt_layer::MakeWriter<'a> for SwappableWriter {
    type Writer = Self;

    #[inline]
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Type alias for the reload handle
// ────────────────────────────────────────────────────────────────────────────

// The key insight from the original working code:
//
//   registry().with(reload::Layer<LevelFilter>).with(fmt_layer)
//
// `reload::Layer<LevelFilter>` sits as a *sibling* layer directly on the
// registry, NOT inside fmt via with_filter().  This is what makes
// rebuild_interest_cache() fire correctly when the handle is modified.
//
// We type-erase the subscriber S with a boxed trait object so that
// RotationHandle can remain S-free and Clone-able.

trait LevelReloader: Send + Sync + 'static {
    fn reload(&self, filter: LevelFilter) -> Result<()>;
}

struct HandleReloader<S>(reload::Handle<LevelFilter, S>);

impl<S: 'static + Send + Sync> LevelReloader for HandleReloader<S> {
    #[inline]
    fn reload(&self, filter: LevelFilter) -> Result<()> {
        self.0.modify(|f| *f = filter).map_err(|e| Error::Reload { source: e })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// RotationHandle
// ────────────────────────────────────────────────────────────────────────────

/// Runtime control surface for a [`RotationLayer`].
///
/// Cheap to clone; every clone shares the same internal state.
#[derive(Clone)]
pub struct RotationHandle {
    reloader: Arc<dyn LevelReloader>,
    writer: SwappableWriter,
    max_files: usize,
}

impl RotationHandle {
    /// Change the minimum log level at runtime.
    ///
    /// `level` is matched case-insensitively.  Unrecognized strings fall back
    /// to `ERROR` with a warning log.
    ///
    /// # Errors
    /// [`Error::NotInit`] if the handle was not properly initialized via
    /// [`global::init`] or [`Builder::build`].
    #[inline]
    pub fn set_level(&self, level: &str) -> Result<()> {
        let new = LevelFilter::from_str(level).unwrap_or_else(|_| {
            tracing::warn!("Unknown log level `{level}`, falling back to ERROR");
            LevelFilter::ERROR
        });
        self.reloader.reload(new)
    }

    /// Redirect log output to a freshly rotated file inside `log_dir`.
    ///
    /// Files are rotated (renamed + oldest deleted) before the new file is
    /// opened, identical to what happens on startup.
    /// Redirect log output to a freshly rotated file inside `log_dir`.
    ///
    ///# Errors
    /// - [`Error::NotInit`] if the handle was not properly initialized via
    ///   [`global::init`] or [`Builder::build`].
    /// - Any I/O error encountered during rotation or file creation
    #[inline]
    pub fn set_path(&self, log_dir: impl AsRef<Path>, log_stem: &str) -> Result<()> {
        let file = rotate_files(log_dir, log_stem, self.max_files)?;
        self.writer.swap(file)
    }
}

impl fmt::Debug for RotationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RotationHandle").field("max_files", &self.max_files).finish_non_exhaustive()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Builder
// ────────────────────────────────────────────────────────────────────────────

/// Builder for [`RotationLayer`].
#[derive(Debug)]
pub struct RotationBuilder {
    pub log_dir: PathBuf,
    pub log_file: String,
    pub max_files: usize,
    pub initial_level: LevelFilter,
}

impl Default for RotationBuilder {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("."),
            log_file: "app.log".into(),
            max_files: 4,
            initial_level: LevelFilter::TRACE,
        }
    }
}
