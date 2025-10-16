//! File version
#[cfg(target_os = "windows")]
use std::path::Path;

/// Represents the version of a file (typically an executable or DLL).
///
/// The fields correspond to the standard four-part Windows version format:
/// `major.minor.patch.build`.
///
/// # Example
///
/// ```
/// let v = Version { major: 10, minor: 0, patch: 19041, build: 1 };
/// assert_eq!(v.to_string(), "10.0.19041.1");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Version {
    /// Major version (e.g., `10` in `10.0.19041.1`)
    pub major: u16,
    /// Minor version (e.g., `0` in `10.0.19041.1`)
    pub minor: u16,
    /// Patch or revision (e.g., `19041` in `10.0.19041.1`)
    pub patch: u16,
    /// Build number (e.g., `1` in `10.0.19041.1`)
    pub build: u16,
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.patch, self.build
        )
    }
}

/// Error type returned when a file's version information cannot be retrieved.
///
/// This error is returned in the following cases:
/// - On **Unix-like systems (Linux, macOS, etc.)**, where executable files
///   generally do not contain Windows-style version information.
/// - On **Windows**, if the file has no `VERSIONINFO` resource or it cannot be parsed.
///
/// # Example
///
/// ```
/// match get_file_version("nonexistent.exe") {
///     Ok(v) => println!("Version: {}", v),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
#[derive(Debug)]
pub struct VersionError;

impl core::fmt::Display for VersionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "file version info not available on this platform or file"
        )
    }
}

impl core::error::Error for VersionError {}

/// Attempts to retrieve the version information of the specified file.
///
/// On **Windows**, this function reads the `VERSIONINFO` resource of an `.exe` or `.dll` file
/// and returns a [`Version`] struct.
/// On **Unix-like systems**, this function always returns [`Err(VersionError)`].
///
/// # Errors
/// If the file has no version info or the platform does not support it.
///
/// # Examples
///
/// ## On Windows
///
/// ```
/// let version = get_file_version("C:\\Windows\\System32\\notepad.exe").unwrap();
/// println!("Version: {}", version); // e.g. "10.0.19041.1"
/// ```
///
/// ## On Linux or macOS
///
/// ```
/// match get_file_version("/bin/ls") {
///     Ok(v) => println!("Version: {}", v),
///     Err(e) => println!("Error: {}", e), // always returns Err on Unix
/// }
/// ```
#[cfg(target_os = "windows")]
pub fn get_file_version<P>(path: P) -> Result<Version, VersionError>
where
    P: AsRef<Path>,
{
    use std::ffi::{c_void, OsStr};
    use std::os::windows::ffi::OsStrExt as _;
    use std::ptr;

    #[link(name = "version")]
    extern "system" {
        fn GetFileVersionInfoSizeW(lptstrFilename: *const u16, lpdwHandle: *mut u32) -> u32;
        fn GetFileVersionInfoW(
            lptstrFilename: *const u16,
            dwHandle: u32,
            dwLen: u32,
            lpData: *mut c_void,
        ) -> i32;
        fn VerQueryValueW(
            pBlock: *const c_void,
            lpSubBlock: *const u16,
            lplpBuffer: *mut *mut c_void,
            puLen: *mut u32,
        ) -> i32;
    }

    #[repr(C)]
    #[allow(non_camel_case_types)]
    #[allow(non_snake_case)]
    struct VS_FIXEDFILEINFO {
        dwSignature: u32,
        dwStrucVersion: u32,
        dwFileVersionMS: u32,
        dwFileVersionLS: u32,
        dwProductVersionMS: u32,
        dwProductVersionLS: u32,
        dwFileFlagsMask: u32,
        dwFileFlags: u32,
        dwFileOS: u32,
        dwFileType: u32,
        dwFileSubtype: u32,
        dwFileDateMS: u32,
        dwFileDateLS: u32,
    }

    let wide: Vec<u16> = path
        .as_ref()
        .as_os_str()
        .encode_wide()
        .chain(core::iter::once(0))
        .collect();

    unsafe {
        let mut handle = 0_u32;
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle);
        if size == 0 {
            return Err(VersionError);
        }

        let mut data = vec![0_u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, data.as_mut_ptr().cast()) == 0 {
            return Err(VersionError);
        }

        let mut lp_buffer: *mut c_void = ptr::null_mut();
        let mut len = 0_u32;
        let sub_block: Vec<u16> = OsStr::new("\\")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        if VerQueryValueW(
            data.as_ptr().cast(),
            sub_block.as_ptr(),
            &mut lp_buffer,
            &mut len,
        ) == 0
        {
            return Err(VersionError);
        }

        let info = &*(lp_buffer as *const VS_FIXEDFILEINFO);
        if info.dwSignature != 0xFEEF04BD {
            return Err(VersionError);
        }

        Ok(Version {
            major: (info.dwFileVersionMS >> 16) as u16,
            minor: (info.dwFileVersionMS & 0xFFFF) as u16,
            patch: (info.dwFileVersionLS >> 16) as u16,
            build: (info.dwFileVersionLS & 0xFFFF) as u16,
        })
    }
}

/// Attempts to retrieve the version information of the specified file.
///
/// On **Unix-like systems**, this function always returns [`Err(VersionError)`].
///
/// # Errors
/// If the file has no version info or the platform does not support it.
///
/// # Examples
///
/// ```
/// match get_file_version("/bin/ls") {
///     Ok(v) => println!("Version: {}", v),
///     Err(e) => println!("Error: {}", e), // always returns Err on Unix
/// }
/// ```
#[cfg(unix)]
pub fn get_file_version<P: AsRef<Path>>(path: P) -> Result<Version, VersionError> {
    Err(VersionError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        #[cfg(windows)]
        {
            let path = "C:\\Windows\\System32\\notepad.exe";
            // let path = "D:/STEAM/steamapps/common/Skyrim Special Edition/Data/../SkyrimSE.exe";
            match get_file_version(path) {
                Ok(v) => println!("{path}: {v}"),
                Err(e) => panic!("Error: {e}"),
            }
        }

        #[cfg(unix)]
        assert!(get_file_version("/bin/ls").is_err());
    }
}
