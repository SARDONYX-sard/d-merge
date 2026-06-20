//! Logging ui
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
use parking_lot::RwLock;
use snafu::prelude::*;

static LOG_DIR_CHANGED: AtomicBool = AtomicBool::new(false);

pub(crate) type LogQueueLock = Arc<RwLock<VecDeque<LogLine>>>;

impl super::App {
    pub(crate) fn start_log_watcher(&mut self) {
        if self.log_watcher_started {
            return;
        }

        let log_path = Path::new(self.settings.log.dir_path.as_str()).join(LOG_FILENAME);
        self.current_log_dir = Some(Path::new(&self.settings.log.dir_path).to_path_buf());

        let log_lines = Arc::clone(&self.log_lines);

        LOG_DIR_CHANGED.store(true, Ordering::Release);
        if let Err(e) = start_log_tail(log_path, log_lines) {
            tracing::error!("log watcher start failed: {e}");
        }

        self.log_watcher_started = true;
    }

    pub(crate) fn update_log_dir(&mut self) {
        let current = Some(PathBuf::from(self.settings.log.dir_path.clone()));

        if current != self.current_log_dir {
            self.current_log_dir = current;
        }
        LOG_DIR_CHANGED.store(true, Ordering::Release);
    }
}

fn start_log_tail(log_path: PathBuf, log_lines: LogQueueLock) -> std::result::Result<(), LogError> {
    thread::spawn(move || {
        if let Err(e) = tail_loop(log_path, log_lines) {
            tracing::error!("tail thread error: {e}");
        }
    });

    Ok(())
}

fn tail_loop(log_path: PathBuf, log_lines: LogQueueLock) -> std::result::Result<(), LogError> {
    let mut reader = BufReader::new(open_file(&log_path)?);
    let mut pos: u64 = 0;

    loop {
        if LOG_DIR_CHANGED.load(Ordering::Acquire) {
            reader = BufReader::new(open_file(&log_path)?);
            pos = 0;
            reader.seek(SeekFrom::End(0)).context(IoSnafu)?;
            LOG_DIR_CHANGED.store(false, Ordering::Release);
        }

        reader.seek(SeekFrom::Start(pos)).context(IoSnafu)?;

        let mut buf = String::new();

        loop {
            buf.clear();

            if reader.read_line(&mut buf).context(IoSnafu)? == 0 {
                break;
            }

            if buf.ends_with('\n') {
                buf.pop();

                if buf.ends_with('\r') {
                    buf.pop();
                }
            }

            push_line(&log_lines, buf.clone());
        }
        // NOTE: Pausing for 1 second prevents the process from freezing due to an excessively high refresh rate
        thread::sleep(Duration::from_secs(1));

        pos = reader.stream_position().context(IoSnafu)?;
    }
}

fn open_file(path: &Path) -> std::result::Result<std::fs::File, LogError> {
    OpenOptions::new().read(true).open(path).context(OpenSnafu { path: path.to_path_buf() })
}

fn push_line(log_lines: &LogQueueLock, line: String) {
    const MAX_LOG_LINES: usize = 600;

    let log_line = LogLine { layout: log_layout(&line), raw: line };

    log_lines.write().push_back(log_line);

    while log_lines.read().len() > MAX_LOG_LINES {
        log_lines.write().pop_front();
    }
}

pub(crate) struct LogLine {
    pub raw: String,
    pub layout: egui::text::LayoutJob,
}

fn log_layout(line: &str) -> egui::text::LayoutJob {
    use egui::{Color32, FontId, TextFormat, text::LayoutJob};

    fn tracing_level_color(level: &str) -> egui::Color32 {
        match level {
            "TRACE" => egui::Color32::from_rgb(180, 0, 255),
            "DEBUG" => egui::Color32::from_rgb(80, 160, 255),
            "INFO" => egui::Color32::from_rgb(0, 200, 83),
            "WARN" => egui::Color32::from_rgb(255, 193, 7),
            "ERROR" => egui::Color32::from_rgb(244, 67, 54),
            _ => egui::Color32::WHITE,
        }
    }

    let mut job = LayoutJob::default();

    let mut parts = line.splitn(3, ' ');
    let time_stamp = parts.next().unwrap_or("");
    let level = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");

    let font_id = FontId::monospace(13.0);
    let normal =
        TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };

    let tracing_color = TextFormat {
        font_id: font_id.clone(),
        color: tracing_level_color(level),
        ..Default::default()
    };

    job.append(time_stamp, 0.0, tracing_color.clone());
    job.append(" ", 0.0, normal.clone());
    job.append(level, 0.0, tracing_color);
    job.append(" ", 0.0, normal);

    // path:line:
    let gray = TextFormat { font_id: font_id.clone(), color: Color32::GRAY, ..Default::default() };
    if let Some(pos) = rest.find(": ") {
        let (location, message) = rest.split_at(pos + 1);
        let location_color =
            TextFormat { font_id, color: Color32::LIGHT_GREEN, ..Default::default() };

        job.append(location, 0.0, location_color);
        job.append(" ", 0.0, gray.clone());
        job.append(message.trim_start(), 0.0, gray);
    } else {
        job.append(rest, 0.0, gray);
    }

    job
}

#[derive(Debug, Snafu)]
pub(crate) enum LogError {
    #[snafu(display("open failed: {source}"))]
    Open { path: PathBuf, source: std::io::Error },

    #[snafu(display("io error: {source}"))]
    Io { source: std::io::Error },
}
