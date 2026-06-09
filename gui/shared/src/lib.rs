pub mod fetch;
pub mod fs;
pub mod i18n;
pub mod log;
pub mod mod_item;
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
    ico_to_rgba(include_bytes!("../../tauri/backend/icons/icon.ico"))
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
