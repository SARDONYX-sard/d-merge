pub mod version;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use std::{io, path::PathBuf};

    use super::Runtime;

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

// NOTE: Be careful not to change the values of `serde` renames arbitrarily.
// These values are used in `settings.json`, so careless changes may break compatibility.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Runtime {
    /// Skyrim Legendary Edition(32bit)
    #[cfg_attr(feature = "serde", serde(rename = "LE"))]
    Le,
    /// Skyrim Special Edition(64bit)
    #[cfg_attr(feature = "serde", serde(rename = "SE"))]
    Se,
    /// Skyrim VR(64bit)
    #[cfg_attr(feature = "serde", serde(rename = "VR"))]
    Vr,
    /// Enderal(32bit)
    #[cfg_attr(feature = "serde", serde(rename = "Enderal"))]
    Enderal,
    /// Enderal Special Edition(64bit)
    #[cfg_attr(feature = "serde", serde(rename = "EnderalSE"))]
    EnderalSe,
}

impl Runtime {
    /// All runtime variants.
    pub const ALL: [Self; 5] = [Self::Le, Self::Se, Self::Vr, Self::Enderal, Self::EnderalSe];

    /// Returns the string representation of this runtime.
    ///
    /// # Examples
    ///
    /// ```
    /// use skyrim_data_dir::Runtime;
    ///
    /// assert_eq!(Runtime::Le.as_str(), "SkyrimLE");
    /// assert_eq!(Runtime::Se.as_str(), "SkyrimSE");
    /// assert_eq!(Runtime::Vr.as_str(), "SkyrimVR");
    /// assert_eq!(Runtime::Enderal.as_str(), "Enderal");
    /// assert_eq!(Runtime::EnderalSe.as_str(), "EnderalSE");
    /// ```
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Le => "SkyrimLE",
            Self::Se => "SkyrimSE",
            Self::Vr => "SkyrimVR",
            Self::Enderal => "Enderal",
            Self::EnderalSe => "EnderalSE",
        }
    }

    /// Get the game executable version.
    ///
    /// Returns `None` if the game executable cannot be found or its file version
    /// cannot be read. On non-Windows platforms, this always returns `None` because
    /// executable file version information is not available.
    ///
    /// # Examples
    ///
    /// ```
    /// let version = skyrim_data_dir::Runtime::Se.get_version("D:/STEAM/steamapps/common/Skyrim Special Edition/Data/");
    /// ```
    pub fn get_version<P>(&self, skyrim_data_dir: P) -> Option<self::version::Version>
    where
        P: AsRef<std::path::Path>,
    {
        let exe = match self {
            Self::Enderal | Self::Le => "TESV.exe",
            Self::EnderalSe | Self::Se => "SkyrimSE.exe",
            Self::Vr => "SkyrimVR.exe",
        };
        let exe_path = skyrim_data_dir.as_ref().parent()?.join(exe);
        self::version::get_file_version(exe_path).ok()
    }
}

impl core::fmt::Display for Runtime {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
