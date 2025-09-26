#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use super::Runtime;
    use std::io;
    use std::path::PathBuf;

    /// Get the skyrim data directory.
    ///
    /// # Errors
    /// Unsupported `get_skyrim_data_dir` on Unix. windows only
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn get_skyrim_data_dir(runtime: Runtime) -> Result<PathBuf, io::Error> {
        let _ = runtime;
        const ERR_MSG: &str = "Unsupported `get_skyrim_data_dir` on Unix. windows only";
        Err(io::Error::new(io::ErrorKind::NotFound, ERR_MSG))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::get_skyrim_data_dir;
#[cfg(target_os = "windows")]
pub use windows::get_skyrim_data_dir;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "UPPERCASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
    /// Skyrim Legendary Edition(32bit)
    Le,
    /// Skyrim Special Edition(64bit)
    Se,
    /// Skyrim VR(64bit)
    Vr,
}

impl Runtime {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Le => "SkyrimLE",
            Self::Se => "SkyrimSE",
            Self::Vr => "SkyrimVR",
        }
    }
}
