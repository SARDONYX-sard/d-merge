use std::{ffi::OsString, io, os::windows::ffi::OsStringExt as _, path::PathBuf};

use super::Runtime;

/// Get the skyrim data directory.
///
/// # Errors
/// Returns an error if the Skyrim directory cannot be found from registry.
#[inline]
pub fn get_skyrim_data_dir(runtime: Runtime) -> Result<PathBuf, io::Error> {
    get_skyrim_dir(runtime).map(|mut path| {
        path.push("Data");
        path
    })
}

fn get_skyrim_dir(runtime: Runtime) -> Result<PathBuf, io::Error> {
    use bindings::*;

    /// `SOFTWARE\\Wow6432Node\\Bethesda Softworks\\Skyrim\0` in UTF-16 LE
    #[rustfmt::skip]
    const SKYRIM_LE_REG_KEY: &[u16] = &[
        0x0053, 0x004F, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005C, // SOFTWARE\
        0x0057, 0x006F, 0x0077, 0x0036, 0x0034, 0x0033, 0x0032, 0x004E, 0x006F, 0x0064, 0x0065, 0x005C, // Wow6432Node\
        // Bethesda Softworks\
        0x0042, 0x0065, 0x0074, 0x0068, 0x0065, 0x0073, 0x0064, 0x0061, 0x0020,
        0x0053, 0x006F, 0x0066, 0x0074, 0x0077, 0x006F, 0x0072, 0x006B, 0x0073, 0x005C,
        0x0053, 0x006B, 0x0079, 0x0072, 0x0069, 0x006D, // Skyrim
        0x0000, // null terminator
    ];

    /// `SOFTWARE\\Bethesda Softworks\\Skyrim Special Edition\0` in UTF-16 LE
    #[rustfmt::skip]
    const SKYRIM_SE_REG_KEY: &[u16] = &[
        0x0053, 0x004F, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005C, // SOFTWARE\
        // Bethesda Softworks\
        0x0042, 0x0065, 0x0074, 0x0068, 0x0065, 0x0073, 0x0064, 0x0061, 0x0020,
        0x0053, 0x006F, 0x0066, 0x0074, 0x0077, 0x006F, 0x0072, 0x006B, 0x0073, 0x005C,
        // Skyrim Special Edition
        0x0053, 0x006B, 0x0079, 0x0072, 0x0069, 0x006D, 0x0020,
        0x0053, 0x0070, 0x0065, 0x0063, 0x0069, 0x0061, 0x006C, 0x0020,
        0x0045, 0x0064, 0x0069, 0x0074, 0x0069, 0x006F, 0x006E,
        // null terminator
        0x0000,
    ];

    /// `SOFTWARE\\Bethesda Softworks\\Skyrim VR` + `\0` in UTF-16 LE
    #[rustfmt::skip]
    const SKYRIM_VR_REG_KEY: &[u16] = &[
        0x0053, 0x004f, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005c, // SOFTWARE\
        0x0042, 0x0065, 0x0074, 0x0068, 0x0065, 0x0073, 0x0064, 0x0061, 0x0020, // Bethesda
        0x0053, 0x006F, 0x0066, 0x0074, 0x0077, 0x006F, 0x0072, 0x006B, 0x0073, 0x005C, // Softworks\
        0x0053, 0x006b, 0x0079, 0x0072, 0x0069, 0x006d, // Skyrim
        0x0020, 0x0056, 0x0052, // VR
        0x0000,
    ];

    /// `SOFTWARE\\SureAI\\Enderal\0` in UTF-16 LE
    #[rustfmt::skip]
    const ENDERAL_REG_KEY: &[u16] = &[
        0x0053, 0x004F, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005C, // SOFTWARE\
        0x0053, 0x0075, 0x0072, 0x0065, 0x0041, 0x0049, 0x005C, // SureAI\
        0x0045, 0x006E, 0x0064, 0x0065, 0x0072, 0x0061, 0x006C, // Enderal
        0x0000, // null terminator
    ];

    /// `SOFTWARE\\SureAI\\EnderalSE\0` in UTF-16 LE
    #[rustfmt::skip]
    const ENDERAL_SE_REG_KEY: &[u16] = &[
        0x0053, 0x004F, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005C, // SOFTWARE\
        0x0053, 0x0075, 0x0072, 0x0065, 0x0041, 0x0049, 0x005C, // SureAI\
        0x0045, 0x006E, 0x0064, 0x0065, 0x0072, 0x0061, 0x006C, 0x0053, 0x0045, // EnderalSE
        0x0000, // null terminator
    ];

    /// `Installed Path\0` in UTF-16 LE
    #[rustfmt::skip]
    const INSTALLED_PATH_VALUE: &[u16] = &[
        0x0049, 0x006E, 0x0073, 0x0074, 0x0061, 0x006C, 0x006C, 0x0065, 0x0064, 0x0020, // Installed
        0x0050, 0x0061, 0x0074, 0x0068, // Path
        0x0000, // null terminator
    ];

    // `Install_Path\0"` in UTF-16 LE
    #[rustfmt::skip]
    const INSTALL_PATH_VALUE: &[u16] = &[
        0x0049, 0x006E, 0x0073, 0x0074, 0x0061, 0x006C, 0x006C, 0x005F, // Install_
        0x0050, 0x0061, 0x0074, 0x0068, // Path
        0x0000,
    ];

    let (root_key, sub_key, value_name, flags) = match runtime {
        Runtime::Le => (
            HKEY_LOCAL_MACHINE,
            SKYRIM_LE_REG_KEY,
            INSTALLED_PATH_VALUE,
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6432KEY,
        ),
        Runtime::Se => (
            HKEY_LOCAL_MACHINE,
            SKYRIM_SE_REG_KEY,
            INSTALLED_PATH_VALUE,
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6432KEY,
        ),
        Runtime::Vr => (
            HKEY_LOCAL_MACHINE,
            SKYRIM_VR_REG_KEY,
            INSTALLED_PATH_VALUE,
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6432KEY,
        ),
        Runtime::Enderal => (HKEY_CURRENT_USER, ENDERAL_REG_KEY, INSTALL_PATH_VALUE, RRF_RT_REG_SZ),
        Runtime::EnderalSe => {
            (HKEY_CURRENT_USER, ENDERAL_SE_REG_KEY, INSTALL_PATH_VALUE, RRF_RT_REG_SZ)
        }
    };

    get_registry_path(root_key, sub_key, value_name, flags)
}

fn get_registry_path(
    root_key: usize,
    sub_key: &[u16],
    value_name: &[u16],
    flags: u32,
) -> Result<PathBuf, io::Error> {
    use bindings::*;

    const MAX_PATH: usize = 4096;
    let mut buffer = vec![0_u16; MAX_PATH];
    let mut data_size = (MAX_PATH * 2) as u32;

    let status = unsafe {
        RegGetValueW(
            root_key,
            sub_key.as_ptr(),
            value_name.as_ptr(),
            flags,
            core::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut data_size,
        )
    };

    if status != ERROR_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status));
    }

    // Convert UTF-16 buffer to PathBuf
    let wide_slice = &buffer[..(data_size as usize / 2)];
    let os_string = OsString::from_wide(wide_slice).to_string_lossy().to_string();
    Ok(PathBuf::from(os_string.trim_end_matches('\0')))
}

#[allow(non_upper_case_globals)]
mod bindings {
    /// - ref: https://docs.rs/windows-sys/latest/windows_sys/Win32/System/Registry/constant.HKEY_LOCAL_MACHINE.html
    pub(super) const HKEY_LOCAL_MACHINE: usize = 0xffffffff80000002;
    /// - ref: https://docs.rs/windows-sys/latest/windows_sys/Win32/System/Registry/constant.HKEY_CURRENT_USER.html
    pub(super) const HKEY_CURRENT_USER: usize = 0xffffffff80000001;
    pub(super) const RRF_SUBKEY_WOW6432KEY: u32 = 0x00020000;
    pub(super) const RRF_RT_REG_SZ: u32 = 0x00000002;
    pub(super) const ERROR_SUCCESS: i32 = 0;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        /// - docs: https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-reggetvaluew
        pub(super) fn RegGetValueW(
            hkey: usize,
            lpSubKey: *const u16,
            lpValue: *const u16,
            dwFlags: u32,
            pdwType: *mut u32,
            pvData: *mut core::ffi::c_void,
            pcbData: *mut u32,
        ) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "Local only"]
    #[test]
    fn test_get_game_dirs() {
        for runtime in [Runtime::Le, Runtime::Se, Runtime::Vr, Runtime::Enderal, Runtime::EnderalSe]
        {
            let result = get_skyrim_data_dir(runtime);
            let _ = dbg!(runtime, result);
        }
    }
}
