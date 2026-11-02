pub mod error;
pub mod fetch;
pub mod fs;
pub mod i18n;
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

pub const CURRENT_VERSION: semver::Version = semver::Version::new(
    parse_u64(env!("CARGO_PKG_VERSION_MAJOR")),
    parse_u64(env!("CARGO_PKG_VERSION_MINOR")),
    parse_u64(env!("CARGO_PKG_VERSION_PATCH")),
);

/// Parses an unsigned decimal integer in a const context.
///
/// Panics at compile time if the input is empty or contains a non-digit.
const fn parse_u64(s: &str) -> u64 {
    let bytes = s.as_bytes();
    assert!(!bytes.is_empty(), "empty version component");

    let mut i = 0;
    let mut acc: u64 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        assert!(b.is_ascii_digit(), "non-digit in version component");
        acc = acc * 10 + (b - b'0') as u64;
        i += 1;
    }
    acc
}
