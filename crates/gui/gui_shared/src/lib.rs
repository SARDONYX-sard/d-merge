/// File dialogs, for the page crates: Linux needs another version with the xdg portal backend.
#[cfg(not(target_os = "linux"))]
pub use rfd;
#[cfg(target_os = "linux")]
pub use rfd_linux as rfd;

pub mod error;
pub mod fetch;
pub mod fs;
pub mod i18n;
pub mod job;
pub mod log;
pub mod patch;
pub mod settings;

/// Get d_merge_icon
///
/// (rgba_data, [width, height])
///
/// # Panics
/// Not icon load
#[inline]
pub fn d_merge_icon() -> (Vec<u8>, [u32; 2]) {
    ico_to_rgba(include_bytes!("../assets/icon.ico"))
}

#[expect(clippy::unwrap_used)]
fn ico_to_rgba(bytes: &[u8]) -> (Vec<u8>, [u32; 2]) {
    let cursor = std::io::Cursor::new(bytes);
    let ico = ico::IconDir::read(cursor).unwrap();
    let entry = ico.entries().first().unwrap();
    let image = entry.decode().unwrap();
    let width = image.width();
    let height = image.height();
    (image.rgba_data().to_vec(), [width, height])
}
