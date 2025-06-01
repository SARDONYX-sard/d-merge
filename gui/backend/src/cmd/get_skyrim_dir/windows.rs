use std::ffi::OsString;
use std::io;
use std::os::windows::ffi::OsStringExt as _;
use std::path::PathBuf;

use crate::cmd::get_skyrim_dir::Runtime;

/// Get the skyrim data directory.
#[inline]
pub fn get_skyrim_data_dir(runtime: Runtime) -> Result<PathBuf, io::Error> {
    get_skyrim_dir(runtime).map(|mut path| {
        path.push("Data");
        path
    })
}

fn get_skyrim_dir(runtime: Runtime) -> Result<PathBuf, io::Error> {
    use bindings::*;

    /// `SOFTWARE\\Bethesda Softworks\\Skyrim Special Edition\0` in UTF-16 LE
    #[rustfmt::skip]
    const SKYRIM_SE_REG_KEY: &[u16] = &[
        0x0053, 0x004F, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005C, // "SOFTWARE\"
        // "Bethesda Softworks\"
        0x0042, 0x0065, 0x0074, 0x0068, 0x0065, 0x0073, 0x0064, 0x0061, 0x0020,
        0x0053, 0x006F, 0x0066, 0x0074, 0x0077, 0x006F, 0x0072, 0x006B, 0x0073, 0x005C,
        // "Skyrim Special Edition"
        0x0053, 0x006B, 0x0079, 0x0072, 0x0069, 0x006D, 0x0020,
        0x0053, 0x0070, 0x0065, 0x0063, 0x0069, 0x0061, 0x006C, 0x0020,
        0x0045, 0x0064, 0x0069, 0x0074, 0x0069, 0x006F, 0x006E,
        // null terminator
        0x0000,
    ];

    /// utf-16 bytes `SOFTWARE\\Bethesda Softworks\\Skyrim VR` + `\0`
    #[rustfmt::skip]
    const SKYRIM_VR_REG_KEY: &[u16] = &[
        0x0053, 0x004f, 0x0046, 0x0054, 0x0057, 0x0041, 0x0052, 0x0045, 0x005c, // SOFTWARE\
        0x0042, 0x0065, 0x0074, 0x0068, 0x0065, 0x0073, 0x0064, 0x0061, 0x0020, // Bethesda
        0x0053, 0x006F, 0x0066, 0x0074, 0x0077, 0x006F, 0x0072, 0x006B, 0x0073, 0x005C, // Softworks\
        0x0053, 0x006b, 0x0079, 0x0072, 0x0069, 0x006d, // Skyrim
        0x0020, 0x0056, 0x0052, // VR
        0x0000,
    ];

    // "Installed Path\0" in UTF-16 LE
    #[rustfmt::skip]
    const VALUE_NAME: &[u16] = &[
        0x0049, 0x006E, 0x0073, 0x0074, 0x0061, 0x006C, 0x006C, 0x0065, 0x0064, 0x0020, // "Installed "
        0x0050, 0x0061, 0x0074, 0x0068, // "Path"
        0x0000, // null terminator
    ];

    let sub_key = match runtime {
        Runtime::Se => SKYRIM_SE_REG_KEY,
        Runtime::Vr => SKYRIM_VR_REG_KEY,
    };

    const MAX_PATH: usize = 4096;
    let mut buffer = vec![0_u16; MAX_PATH];
    let mut data_size = (MAX_PATH * 2) as u32;

    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            sub_key.as_ptr(),
            VALUE_NAME.as_ptr(),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6432KEY,
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
    let os_string = OsString::from_wide(wide_slice)
        .to_string_lossy()
        .to_string();
    Ok(PathBuf::from(os_string.trim_end_matches('\0')))
}

#[allow(non_upper_case_globals)]
mod bindings {
    /// -ref: https://docs.rs/windows-sys/latest/windows_sys/Win32/System/Registry/constant.HKEY_LOCAL_MACHINE.html
    pub const HKEY_LOCAL_MACHINE: usize = 0xffffffff80000002;
    pub const RRF_SUBKEY_WOW6432KEY: u32 = 0x00020000;
    pub const RRF_RT_REG_SZ: u32 = 0x00000002;
    pub const ERROR_SUCCESS: i32 = 0;

    #[link(name = "advapi32")]
    extern "system" {
        /// - docs: https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-reggetvaluew
        pub fn RegGetValueW(
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
    fn test_get_skyrim_dir() {
        let path = get_skyrim_data_dir(Runtime::Se).unwrap_or_else(|e| panic!("{e}"));
        dbg!(path); // == "D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data"
    }
}
