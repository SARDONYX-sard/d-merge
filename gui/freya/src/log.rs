//! Log file watcher.
//!
//! A thread continuously tails the log file and pushes the lines into [`LogQueueLock`].
use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::{BufRead as _, BufReader, Seek as _, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use d_merge_gui_shared::log::LOG_FILENAME;
use freya::prelude::*;
use parking_lot::RwLock;
use snafu::prelude::*;

/// Set to re-open the log file from the start of the tail loop.
static LOG_DIR_CHANGED: AtomicBool = AtomicBool::new(false);

/// Path of the log file being tailed.
static LOG_PATH: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Keep only the latest lines.
const MAX_LOG_LINES: usize = 600;

pub(crate) type LogQueueLock = Arc<RwLock<VecDeque<String>>>;

/// Starts the log tail thread for `<log_dir>/d_merge.log`.
pub(crate) fn start_log_watcher(log_dir: &str, log_lines: LogQueueLock) {
    change_log_dir(log_dir);

    thread::spawn(move || {
        if let Err(e) = tail_loop(&log_lines) {
            tracing::error!("tail thread error: {e}");
        }
    });
}

/// Makes the tail thread re-open `<log_dir>/d_merge.log`. (e.g. after the log file was moved)
pub(crate) fn change_log_dir(log_dir: &str) {
    *LOG_PATH.write() = Some(Path::new(log_dir).join(LOG_FILENAME));
    LOG_DIR_CHANGED.store(true, Ordering::Release);
}

fn tail_loop(log_lines: &LogQueueLock) -> Result<(), LogError> {
    let mut reader: Option<BufReader<std::fs::File>> = None;
    let mut pos: u64 = 0;
    let mut buf = String::new();

    loop {
        if LOG_DIR_CHANGED.swap(false, Ordering::AcqRel) {
            reader = None;
            pos = 0;
        }

        // NOTE: The log file may not exist yet, so retry opening it until it does.
        if reader.is_none() {
            let path = LOG_PATH.read().clone();
            reader = path.and_then(|path| open_file(&path).ok()).map(BufReader::new);
        }

        let Some(reader) = reader.as_mut() else {
            thread::sleep(Duration::from_secs(1));
            continue;
        };

        reader.seek(SeekFrom::Start(pos)).context(IoSnafu)?;

        loop {
            buf.clear();

            if reader.read_line(&mut buf).context(IoSnafu)? == 0 {
                break;
            }

            let line = buf.trim_end_matches(['\n', '\r']);
            push_line(log_lines, line.to_string());
        }
        // NOTE: Pausing for 1 second prevents the process from freezing due to an excessively high refresh rate
        thread::sleep(Duration::from_secs(1));

        pos = reader.stream_position().context(IoSnafu)?;
    }
}

fn open_file(path: &Path) -> Result<std::fs::File, LogError> {
    OpenOptions::new().read(true).open(path).context(OpenSnafu { path: path.to_path_buf() })
}

fn push_line(log_lines: &LogQueueLock, line: String) {
    let mut lines = log_lines.write();
    lines.push_back(line);
    let excess = lines.len().saturating_sub(MAX_LOG_LINES);
    lines.drain(..excess);
}

/// Renders one log line: `<timestamp> <LEVEL> <path:line:> <message>`
pub(crate) fn log_line(line: &str) -> Element {
    const GRAY: Color = Color::from_rgb(160, 160, 160);
    const LOCATION: Color = Color::from_rgb(144, 238, 144);

    fn level_color(level: &str) -> Color {
        match level {
            "TRACE" => Color::from_rgb(180, 0, 255),
            "DEBUG" => Color::from_rgb(80, 160, 255),
            "INFO" => Color::from_rgb(0, 200, 83),
            "WARN" => Color::from_rgb(255, 193, 7),
            "ERROR" => Color::from_rgb(244, 67, 54),
            _ => Color::WHITE,
        }
    }

    let mut parts = line.splitn(3, ' ');
    let time_stamp = parts.next().unwrap_or("");
    let level = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    let level_color = level_color(level);

    let mut spans = vec![
        Span::new(time_stamp.to_string()).color(level_color),
        Span::new(" ").color(GRAY),
        Span::new(level.to_string()).color(level_color),
        Span::new(" ").color(GRAY),
    ];

    if let Some(pos) = rest.find(": ") {
        let (location, message) = rest.split_at(pos + 1);
        spans.push(Span::new(location.to_string()).color(LOCATION));
        spans.push(Span::new(" ").color(GRAY));
        spans.push(Span::new(message.trim_start().to_string()).color(GRAY));
    } else {
        spans.push(Span::new(rest.to_string()).color(GRAY));
    }

    paragraph().width(Size::fill()).font_size(13.0).spans_iter(spans.into_iter()).into_element()
}

#[derive(Debug, Snafu)]
pub(crate) enum LogError {
    #[snafu(display("open failed({}): {source}", path.display()))]
    Open { path: PathBuf, source: std::io::Error },

    #[snafu(display("io error: {source}"))]
    Io { source: std::io::Error },
}
