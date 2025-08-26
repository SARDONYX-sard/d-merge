use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

pub const LOG_DIR: &str = "./.d_merge/logs/";

/// Start a log file watcher thread.
///
/// This continuously updates `log_lines` with the latest contents of the log file.
pub fn start_log_watcher(log_lines: Arc<Mutex<Vec<String>>>) {
    let mut log_path = Path::new(LOG_DIR).to_path_buf();
    log_path.push("d_merge.log");

    thread::spawn(move || {
        let file = File::open(&log_path).unwrap_or_else(|_| File::create(&log_path).unwrap());
        let reader = BufReader::new(file);

        for line in reader.lines().map_while(Result::ok) {
            log_lines.lock().unwrap().push(line);
        }

        let (tx, rx) = mpsc::channel();
        let config = Config::default().with_poll_interval(Duration::from_secs(1));
        let mut watcher: RecommendedWatcher =
            Watcher::new(tx, config).expect("Failed to create watcher");
        watcher
            .watch(&log_path, RecursiveMode::NonRecursive)
            .expect("Failed to watch log file");

        loop {
            if rx.recv().is_ok() {
                if let Ok(file) = File::open(&log_path) {
                    let reader = BufReader::new(file);
                    let lines: Vec<_> = reader.lines().map_while(Result::ok).collect();
                    *log_lines.lock().unwrap() = lines;
                }
            }
        }
    });
}
