//! # tracing-rotation
//!
//! A [`tracing_subscriber::Layer`] that writes to rotating log files, with
//! runtime-adjustable log level and log-file path.
//!
//! Two APIs are provided:
//!
//! ## Layer API (composable)
//!
//! ```rust,no_run
//! use tracing_rotation::RotationLayer;
//! use tracing_subscriber::prelude::*;
//!
//! let (layer, handle) = RotationLayer::builder()
//!     .log_dir("/var/log/myapp")
//!     .log_stem("app.log")
//!     .max_files(5)
//!     .build()
//!     .expect("failed to create rotation layer");
//!
//! tracing_subscriber::registry()
//!     .with(layer)   // compose freely with other layers
//!     .init();
//!
//! handle.set_level("debug").unwrap();
//! handle.set_path("/tmp/myapp", "app.log").unwrap();
//! ```
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
pub mod rotate;

use std::{
    fmt,
    fs::File,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use parking_lot::Mutex;
use tracing::{Event, Level, Subscriber, metadata::LevelFilter};
use tracing_subscriber::{Layer, fmt as fmt_layer, layer::Context, registry::LookupSpan, reload};

use crate::{
    error::{Error, Result},
    rotate::rotate_files,
};

// ────────────────────────────────────────────────────────────────────────────
// SwappableWriter
// ────────────────────────────────────────────────────────────────────────────

/// An [`io::Write`] + [`fmt_layer::MakeWriter`] whose underlying [`File`] can
/// be replaced at runtime without rebuilding the subscriber stack.
#[derive(Clone)]
#[expect(missing_debug_implementations)]
pub struct SwappableWriter(Arc<Mutex<BufWriter<File>>>);

impl SwappableWriter {
    #[inline]
    fn new(file: File) -> Self {
        Self(Arc::new(Mutex::new(BufWriter::new(file))))
    }

    #[inline]
    pub(crate) fn swap(&self, file: File) -> Result<()> {
        let mut guard = self.0.lock();
        guard.flush().ok();
        *guard = BufWriter::new(file);
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
// RotationLayer
// ────────────────────────────────────────────────────────────────────────────

/// A pair of layers to be registered together on the subscriber.
///
/// Because `reload::Layer<LevelFilter>` must be a **sibling** of the fmt layer
/// (not nested inside it via `with_filter`) for level changes to take effect,
/// `RotationLayer` is itself a thin delegating [`Layer`] that carries both
/// the reload filter and the fmt writer internally.
///
/// When you call `.with(rotation_layer)` the single `with()` call registers
/// this delegating shell, which internally applies both filter and formatting.
pub struct RotationLayer<S> {
    /// The level-filter half.  Lives as a sibling, not inside the fmt layer.
    filter: Arc<Mutex<reload::Layer<LevelFilter, S>>>,
    /// The fmt writer half.
    fmt: Box<dyn Layer<S> + Send + Sync + 'static>,
}

impl<S> fmt::Debug for RotationLayer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RotationLayer").finish_non_exhaustive()
    }
}

impl<S> Layer<S> for RotationLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // NOTE: It appears to be an undocumented API(tracing-subscriber = "0.3.23"),
    // but if we don't use it, a bug occurs that prevents errors from being logged.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.filter.lock().max_level_hint()
    }

    #[inline]
    fn enabled(&self, metadata: &tracing::Metadata<'_>, ctx: Context<'_, S>) -> bool {
        self.filter.lock().enabled(metadata, ctx.clone())
    }

    #[inline]
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        self.fmt.on_event(event, ctx);
    }

    #[inline]
    fn on_enter(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        self.fmt.on_enter(id, ctx);
    }

    #[inline]
    fn on_exit(&self, id: &tracing::span::Id, ctx: Context<'_, S>) {
        self.fmt.on_exit(id, ctx);
    }

    #[inline]
    fn on_close(&self, id: tracing::span::Id, ctx: Context<'_, S>) {
        self.fmt.on_close(id, ctx);
    }

    #[inline]
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        self.fmt.on_new_span(attrs, id, ctx);
    }

    #[inline]
    fn on_record(
        &self,
        span: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        self.fmt.on_record(span, values, ctx);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Builder
// ────────────────────────────────────────────────────────────────────────────

/// Builder for [`RotationLayer`].
#[derive(Debug)]
pub struct Builder {
    log_dir: PathBuf,
    log_file: String,
    max_files: usize,
    initial_level: LevelFilter,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("."),
            log_file: "app.log".into(),
            max_files: 4,
            initial_level: LevelFilter::TRACE,
        }
    }
}

impl Builder {
    /// Directory in which log files are stored (created if absent).
    #[inline]
    pub fn log_dir<P>(mut self, dir: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.log_dir = dir.into();
        self
    }

    /// Base file name with extension.
    /// The live file keeps this exact name; rotated copies are stored as `{stem}_{YYYY-MM-DD_HH-MM-SS}.{ext}`.
    #[inline]
    pub fn log_file_name<S: Into<String>>(mut self, name: S) -> Self {
        self.log_file = name.into();
        self
    }

    /// Maximum number of files (live + archived) retained on disk.
    /// Values below 1 are silently clamped to 1.
    #[inline]
    pub fn max_files(mut self, n: usize) -> Self {
        self.max_files = n.max(1);
        self
    }

    /// Initial [`LevelFilter`].  Defaults to `TRACE`.
    #[inline]
    pub fn level(mut self, level: impl Into<Level>) -> Self {
        self.initial_level = level.into().into();
        self
    }

    /// Consume the builder and produce a `(layer, handle)` pair.
    ///
    /// The produced [`RotationLayer`] replicates the structure of the original
    /// working code:
    ///
    /// ```text
    /// registry
    ///   └── RotationLayer
    ///         ├── reload::Layer<LevelFilter>   ← sibling, not nested in fmt
    ///         └── fmt::Layer<SwappableWriter>
    /// ```
    ///
    /// Placing `reload::Layer` as a sibling (not inside `with_filter()`) is
    /// what allows `Handle::modify()` to correctly invalidate tracing's
    /// call-site interest cache so level changes take effect immediately.
    ///
    /// # Errors
    /// Fails if the log directory cannot be created or the initial file cannot
    /// be opened.
    pub fn build<S>(self) -> Result<(RotationLayer<S>, RotationHandle)>
    where
        S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static,
    {
        let file = rotate_files(&self.log_dir, &self.log_file, self.max_files)?;
        let writer = SwappableWriter::new(file);

        // NOTE: It is important to keep `reload` and `fmt` separate. Otherwise, the level changer will not work.
        let (reload_layer, reload_handle) = reload::Layer::new(self.initial_level);

        let fmt = fmt_layer::layer::<S>()
            .compact()
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .with_writer(writer.clone());

        let handle = RotationHandle {
            reloader: Arc::new(HandleReloader(reload_handle)),
            writer,
            max_files: self.max_files,
        };

        let layer =
            RotationLayer { filter: Arc::new(Mutex::new(reload_layer)), fmt: Box::new(fmt) };

        Ok((layer, handle))
    }
}

impl RotationLayer<()> {
    /// Create a [`Builder`] with sensible defaults.
    #[inline]
    pub fn builder() -> Builder {
        Builder::default()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tracing_subscriber::{Registry, prelude::*};

    use super::*;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn file_count(dir: &Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| !t.is_dir()))
            .count()
    }

    #[test]
    fn layer_composes_with_registry() -> TestResult {
        let tmp = temp_dir::TempDir::new()?;
        let (layer, _handle): (RotationLayer<_>, _) = RotationLayer::builder()
            .log_dir(tmp.path())
            .log_file_name("test.log")
            .max_files(3)
            .build()?;

        let _ = tracing_subscriber::registry().with(layer);
        Ok(())
    }

    #[test]
    fn handle_set_level_and_set_path() -> TestResult {
        let tmp1 = temp_dir::TempDir::new()?;
        let tmp2 = temp_dir::TempDir::new()?;

        let (layer, handle): (RotationLayer<_>, _) = RotationLayer::builder()
            .log_dir(tmp1.path())
            .log_file_name("app.log")
            .max_files(4)
            .build()?;

        let _guard = tracing_subscriber::registry().with(layer).set_default();

        handle.set_level("info")?;
        handle.set_path(tmp2.path(), "app.log")?;

        tracing::info!("written to the new path");
        assert_eq!(file_count(tmp2.path()), 1);
        Ok(())
    }

    #[test]
    fn handle_is_clone_and_debug() -> TestResult {
        let tmp = temp_dir::TempDir::new()?;
        let (_, handle): (RotationLayer<Registry>, _) =
            RotationLayer::builder().log_dir(tmp.path()).log_file_name("app.log").build()?;

        let cloned = handle;
        let _ = format!("{cloned:?}");
        Ok(())
    }
}
