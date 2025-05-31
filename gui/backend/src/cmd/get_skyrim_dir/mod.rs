#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use super::Runtime;
    use std::path::PathBuf;

    /// # Note
    /// Unsupported `get_skyrim_data_dir` on Unix. windows only
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn get_skyrim_data_dir(runtime: Runtime) -> Option<PathBuf> {
        let _ = runtime;
        tracing::info!("Unsupported `get_skyrim_data_dir` on Unix. windows only");
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::get_skyrim_data_dir;
#[cfg(target_os = "windows")]
pub use windows::get_skyrim_data_dir;

#[derive(Debug, Clone, Copy)]
pub enum Runtime {
    Se,
    Vr,
}
