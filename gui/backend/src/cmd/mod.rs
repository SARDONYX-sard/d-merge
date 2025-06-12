pub(crate) mod conversion;
pub(crate) mod fs;
pub(crate) mod get_skyrim_dir;
pub(crate) mod log;
pub(crate) mod patch;

use std::sync::atomic::AtomicBool;
use tauri::{Emitter as _, Window};

pub static IS_VFS_MODE: AtomicBool = AtomicBool::new(false);
#[tauri::command]
pub(crate) fn set_vfs_mode(value: bool) {
    use std::sync::atomic::Ordering;

    IS_VFS_MODE.store(value, Ordering::Release);
}

/// Create closure that reports.
pub(super) fn sender<S>(window: Window, event: &'static str) -> impl Fn(S) + Clone
where
    S: serde::Serialize + Clone,
{
    move |payload: S| {
        if let Err(err) = window.emit(event, payload) {
            tracing::error!("{}", err);
        };
    }
}

/// Early return with Err() and write log error.
macro_rules! bail {
    ($err:expr) => {{
        tracing::error!("{}", $err);
        return Err($err.to_string());
    }};
}
pub(super) use bail;

/// Measure the elapsed time and return the result of the given asynchronous function.
macro_rules! time {
    ($name:literal, $expr:expr) => {{
        let start = std::time::Instant::now();
        let res = $expr.or_else(|err| bail!(err));
        let elapsed = start.elapsed();
        tracing::info!(
            "{} time: {}.{}s.",
            $name,
            elapsed.as_secs(),
            elapsed.subsec_millis()
        );
        res
    }};
}
pub(super) use time;
