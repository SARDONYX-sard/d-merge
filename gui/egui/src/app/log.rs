use std::{
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

impl super::App {
    pub(crate) fn start_log_watcher(&mut self, ctx: &egui::Context) {
        if self.log_watcher_started {
            return;
        }

        let log_path = Path::new(self.settings.log.dir_path.as_str()).join(LOG_FILENAME);
        self.current_log_dir = Some(Path::new(&self.settings.log.dir_path).to_path_buf());

        let log_lines = Arc::clone(&self.log_lines);
        let ctx = ctx.clone();

        LOG_DIR_CHANGED.store(true, Ordering::Release);
        if let Err(e) = start_log_tail(log_path, log_lines, ctx) {
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

fn start_log_tail(
    log_path: PathBuf,
    log_lines: Arc<RwLock<Vec<String>>>,
    ctx: egui::Context,
) -> std::result::Result<(), LogError> {
    thread::spawn(move || {
        if let Err(e) = tail_loop(log_path, log_lines, ctx) {
            tracing::error!("tail thread error: {e}");
        }
    });

    Ok(())
}

fn tail_loop(
    log_path: PathBuf,
    log_lines: Arc<RwLock<Vec<String>>>,
    ctx: egui::Context,
) -> std::result::Result<(), LogError> {
    let mut reader = BufReader::new(open_file(&log_path)?);
    let mut pos: u64 = 0;

    loop {
        // dir change trigger
        if LOG_DIR_CHANGED.load(Ordering::Acquire) {
            reader = BufReader::new(open_file(&log_path)?);
            pos = 0;
            reader.seek(SeekFrom::End(pos as i64)).context(IoSnafu)?;
            LOG_DIR_CHANGED.store(false, Ordering::Release);
        }
        reader.seek(SeekFrom::Start(pos)).context(IoSnafu)?;

        let mut new_line = false;

        for line in std::io::Read::by_ref(&mut reader).lines() {
            let line = line.context(IoSnafu)?;
            push_line(&log_lines, line);
            new_line = true;
        }

        pos = reader.stream_position().context(IoSnafu)?;

        if new_line {
            ctx.request_repaint();
        }

        thread::sleep(Duration::from_millis(100));
    }
}

fn open_file(path: &Path) -> std::result::Result<std::fs::File, LogError> {
    OpenOptions::new().read(true).open(path).context(OpenSnafu { path: path.to_path_buf() })
}

fn push_line(log_lines: &Arc<RwLock<Vec<String>>>, line: String) {
    const MAX_LOG_LINES: usize = 10_000;

    let mut lines = log_lines.write();
    lines.push(line);

    if lines.len() > MAX_LOG_LINES {
        let drain_len = lines.len() - MAX_LOG_LINES;
        lines.drain(0..drain_len);
    }
}

#[derive(Debug, Snafu)]
pub(crate) enum LogError {
    #[snafu(display("open failed: {source}"))]
    Open { path: PathBuf, source: std::io::Error },

    #[snafu(display("io error: {source}"))]
    Io { source: std::io::Error },
}
