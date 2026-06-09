use std::{
    fs::OpenOptions,
    io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::Duration,
};

use parking_lot::RwLock;
use snafu::ResultExt as _;

use crate::app::App;

impl App {
    /// Starts the log-tail watcher on the first frame.
    ///
    /// Subsequent calls are no-ops (`log_watcher_started` guards the spawn).
    /// The watcher writes to [`App::log_lines`] and requests a repaint via
    /// the cloned [`egui::Context`].
    pub(crate) fn start_log_watcher(&mut self, ctx: &egui::Context) {
        if self.log_watcher_started {
            return;
        }

        let log_dir = self.settings.log.dir_path.as_str();
        let log_path = Path::new(log_dir).join(d_merge_gui_shared::log::LOG_FILENAME);

        let log_lines = Arc::clone(&self.log_lines);
        let ctx = ctx.clone();
        if let Err(err) = start_log_tail(&log_path, log_lines, Some(ctx)) {
            tracing::error!("Couldn't start log watcher: {err}");
        }

        self.log_watcher_started = true;
    }
}

/// Maximum number of log entries to retain (older entries are automatically discarded)
const MAX_LOG_LINES: usize = 10_000;

/// log file & Starts tail thread
///
/// # Errors
/// If fail to canonicalize log path.
fn start_log_tail(
    log_path: &Path,
    log_lines: Arc<RwLock<Vec<String>>>,
    ctx: Option<egui::Context>,
) -> Result<()> {
    let log_path = log_path.canonicalize().context(CanonicalizeSnafu { path: log_path })?;

    thread::spawn(move || {
        if let Err(e) = tail_loop(log_path, log_lines, ctx) {
            tracing::error!("log tail thread exited with error: {e}");
        }
    });

    Ok(())
}

fn tail_loop(
    log_path: PathBuf,
    log_lines: Arc<RwLock<Vec<String>>>,
    ctx: Option<egui::Context>,
) -> Result<()> {
    // #[allow(clippy::suspicious_open_options)]
    let file = OpenOptions::new()
        .read(true)
        .open(&log_path)
        .with_context(|_| OpenSnafu { path: log_path })?;

    let mut reader = BufReader::new(file);
    let mut pos = reader.seek(SeekFrom::End(0)).context(IoSnafu)?;

    loop {
        reader.seek(SeekFrom::Start(pos)).context(IoSnafu)?;

        let mut new_line = false;
        for line in reader.by_ref().lines() {
            let line = line.context(IoSnafu)?;
            push_line(&log_lines, line);
            new_line = true;
        }

        pos = reader.stream_position().context(IoSnafu)?;

        if new_line && let Some(ref ctx) = ctx {
            ctx.request_repaint();
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// inner fn：ring buffer push
fn push_line(log_lines: &Arc<RwLock<Vec<String>>>, line: String) {
    let mut lines = log_lines.write();
    lines.push(line);
    let len = lines.len();
    if len > MAX_LOG_LINES {
        lines.drain(0..(len - MAX_LOG_LINES));
    }
}

#[derive(Debug, snafu::Snafu)]
pub(crate) enum LogError {
    #[snafu(display("Failed to open log file at {path:?}: {source}"))]
    Open { path: PathBuf, source: std::io::Error },

    #[snafu(display("I/O error while reading log file: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("Failed to canonicalize path {path:?}: {source}"))]
    Canonicalize { path: PathBuf, source: std::io::Error },
}

type Result<T, E = LogError> = std::result::Result<T, E>;
