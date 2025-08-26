/// Start a log file watcher thread.
///
/// This continuously updates `log_lines` with the latest contents of the log file.
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Read as _, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

/// Maximum number of log entries to retain (older entries are automatically discarded)
const MAX_LOG_LINES: usize = 10_000;
pub const LOG_DIR: &str = "./.d_merge/logs/";

/// inner fn：ring buffer push
fn push_line(log_lines: &Arc<Mutex<Vec<String>>>, line: String) {
    let mut lines = log_lines.lock().unwrap();
    lines.push(line);
    let len = lines.len();
    if len > MAX_LOG_LINES {
        lines.drain(0..(len - MAX_LOG_LINES));
    }
}

/// log file & Starts tail thread
pub fn start_log_tail(log_lines: Arc<Mutex<Vec<String>>>, ctx: Option<egui::Context>) {
    let mut log_path = Path::new(LOG_DIR).to_path_buf();
    log_path.push("d_merge.log");
    let log_path = log_path.canonicalize().unwrap();

    thread::spawn(move || {
        #[allow(clippy::suspicious_open_options)]
        let file = OpenOptions::new()
            .read(true)
            .open(&log_path)
            .expect("Failed to open log file");

        let mut reader = BufReader::new(file);
        // from tail
        let mut pos = reader.seek(SeekFrom::End(0)).unwrap();

        loop {
            reader.seek(SeekFrom::Start(pos)).unwrap();

            let mut new_line = false;
            for line in reader.by_ref().lines().map_while(Result::ok) {
                push_line(&log_lines, line);
                new_line = true;
            }

            // store last pos
            pos = reader.stream_position().unwrap();

            if new_line {
                if let Some(ref ctx) = ctx {
                    ctx.request_repaint();
                }
            }

            thread::sleep(Duration::from_millis(100)); // poring
        }
    });
}
