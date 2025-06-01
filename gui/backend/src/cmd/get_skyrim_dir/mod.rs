#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use super::Runtime;
    use std::io;
    use std::path::PathBuf;

    /// # Note
    /// Unsupported `get_skyrim_data_dir` on Unix. windows only
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn get_skyrim_data_dir(runtime: Runtime) -> Result<PathBuf, io::Error> {
        let _ = runtime;
        const ERR_MSG: &str = "Unsupported `get_skyrim_data_dir` on Unix. windows only";
        tracing::info!(ERR_MSG);
        Err(io::Error::new(io::ErrorKind::NotFound, ERR_MSG))
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
