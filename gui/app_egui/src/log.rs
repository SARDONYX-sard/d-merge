//! Start a log file watcher thread.
//!
//! This continuously updates `log_lines` with the latest contents of the log file.
use snafu::ResultExt as _;
use std::{
    fs::OpenOptions,
    io::{BufRead as _, BufReader, Read as _, Seek as _, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

/// Maximum number of log entries to retain (older entries are automatically discarded)
const MAX_LOG_LINES: usize = 10_000;
pub const LOG_FILENAME: &str = "d_merge.log";

pub fn get_log_dir<P>(output_dir: P) -> PathBuf
where
    P: AsRef<Path>,
{
    output_dir.as_ref().join(".d_merge/logs")
}

/// log file & Starts tail thread
///
/// # Errors
/// If fail to canonicalize log path.
pub fn start_log_tail(
    log_path: &Path,
    log_lines: Arc<Mutex<Vec<String>>>,
    ctx: Option<egui::Context>,
) -> Result<()> {
    let log_path = log_path
        .canonicalize()
        .context(CanonicalizeSnafu { path: log_path })?;

    thread::spawn(move || {
        if let Err(e) = tail_loop(log_path, log_lines, ctx) {
            tracing::error!("log tail thread exited with error: {e}");
        }
    });

    Ok(())
}

fn tail_loop(
    log_path: PathBuf,
    log_lines: Arc<Mutex<Vec<String>>>,
    ctx: Option<egui::Context>,
) -> Result<()> {
    #[allow(clippy::suspicious_open_options)]
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

        if new_line {
            if let Some(ref ctx) = ctx {
                ctx.request_repaint();
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// inner fn：ring buffer push
fn push_line(log_lines: &Arc<Mutex<Vec<String>>>, line: String) {
    let mut lines = log_lines.lock().unwrap();
    lines.push(line);
    let len = lines.len();
    if len > MAX_LOG_LINES {
        lines.drain(0..(len - MAX_LOG_LINES));
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum LogError {
    #[snafu(display("Failed to open log file at {path:?}: {source}"))]
    Open {
        path: PathBuf,
        source: std::io::Error,
    },

    #[snafu(display("I/O error while reading log file: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("Failed to canonicalize path {path:?}: {source}"))]
    Canonicalize {
        path: PathBuf,
        source: std::io::Error,
    },
}

type Result<T, E = LogError> = std::result::Result<T, E>;
