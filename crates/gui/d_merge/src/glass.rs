//! Glass, an option of custom themes (`glass` of a preset): translucent colors over a blurred backdrop,
//! like macOS from Big Sur on. The colors are the preset's, e.g. `themes/frosted_dark_glass.json`.
//!
//! ```text
//! ┌ title bar ───────────────────────────────────────────┐
//! │ page bar         <- `panel_fill`: thin, the backdrop    │
//! ├────────────┬───────────────────────────────────────────┤    shows through
//! │ side bar   │ editor, tables  <- `extreme_bg_color`:    │
//! │            │                    nearly opaque, to read  │
//! │            │   ┌ popup ────────┐ <- `window_fill`:     │
//! │            │   │               │    thicker, with a    │
//! │            │   └───────────────┘    soft shadow        │
//! └────────────┴───────────────────────────────────────────┘
//!   behind all of it: `glass_tint` over the blurred backdrop
//! ```
//!
//! How much shows through is the alpha of each color, and of `glass_tint`: e.g. `#00000000` leaves the
//! backdrop only blurred, `#000000C0` darkens it a lot.
//!
//! # The backdrop
//! egui can't blur what is behind a panel, so the blur comes from elsewhere:
//! - a background image, if one is set: blurred once when it is loaded, see [`blur_wallpaper`], and drawn
//!   in place of the image.
//! - else the desktop behind the window, blurred by Windows: see [`OsBackdrop`].
//! - else (Linux) nothing is blurred: the colors are over a plain one, see [`base`].
//!
//! What lies over a panel inside the app, like a popup over the editor, isn't blurred either: a thicker
//! `window_fill` keeps what is under it from showing through too much.

use egui::{Color32, Shadow};

/// The color under everything when there is no backdrop to show.
pub(crate) const fn base(dark: bool) -> Color32 {
    if dark { Color32::from_rgb(32, 32, 36) } else { Color32::from_rgb(232, 232, 237) }
}

/// The large soft shadow of windows, which lifts them off what is under them.
pub(crate) const fn window_shadow(dark: bool) -> Shadow {
    Shadow { offset: [0, 12], blur: 36, spread: 0, color: Color32::from_black_alpha(if dark { 110 } else { 46 }) }
}

/// The long side of the image that is blurred: the blur is so strong that the detail of a larger one
/// would be lost anyway, and the GPU scales it up smoothly.
const BLUR_IMAGE_SIZE: u32 = 256;
/// The blur at [`BLUR_IMAGE_SIZE`]: about 45 pixels of a full HD image, as heavy as macOS's.
const BLUR_SIGMA: f32 = 6.0;
/// How much more saturated the blurred image is: macOS's materials make the colors behind them richer.
const SATURATION: f32 = 1.35;

/// The background image as glass shows it: scaled down, blurred and saturated.
///
/// e.g. 3840x2160 -> 256x144, which takes a few milliseconds once, when the image is loaded.
pub(crate) fn blur_wallpaper(image: &image::RgbaImage) -> egui::ColorImage {
    let (width, height) = image.dimensions();
    let scale = f64::from(BLUR_IMAGE_SIZE) / f64::from(width.max(height).max(1));
    let size = |side: u32| ((f64::from(side) * scale).round() as u32).max(1);
    let small = image::imageops::resize(image, size(width), size(height), image::imageops::FilterType::Triangle);
    let mut blurred = image::imageops::blur(&small, BLUR_SIGMA);
    for pixel in blurred.pixels_mut() {
        saturate(&mut pixel.0, SATURATION);
    }
    let size = [blurred.width() as usize, blurred.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, blurred.as_raw())
}

/// Moves the color away from its gray by `factor`. e.g. `factor` 1.0 leaves it as it is.
fn saturate(rgba: &mut [u8; 4], factor: f32) {
    let [r, g, b, _] = rgba.map(f32::from);
    let gray = 0.0722_f32.mul_add(b, 0.2126_f32.mul_add(r, 0.7152 * g));
    for channel in &mut rgba[..3] {
        let value = (f32::from(*channel) - gray).mul_add(factor, gray);
        *channel = value.round().clamp(0.0, 255.0) as u8;
    }
}

/// The desktop behind the window, blurred by Windows and tinted with `glass_tint`, kept as the theme
/// asks: applied, tinted anew, taken away.
///
/// This is Windows' blur behind (Windows 10 1809 and later, and 11), not its acrylic: the acrylic of
/// Windows 11 always lays its own tint over the desktop, so how much shows through could not be chosen,
/// and the acrylic of Windows 10 makes moving the window lag.
#[derive(Debug, Default)]
pub(crate) struct OsBackdrop {
    /// The tint applied, as straight (not premultiplied) RGBA. `None`: no blur.
    applied: Option<[u8; 4]>,
    /// The OS can't: not asked again.
    unsupported: bool,
}

impl OsBackdrop {
    /// Is the desktop behind the window showing, so that the window's own background must be clear?
    pub(crate) const fn is_shown(&self) -> bool {
        self.applied.is_some()
    }

    /// Applies `tint` (`None`: no blur) if it isn't already.
    pub(crate) fn sync(&mut self, frame: &eframe::Frame, tint: Option<Color32>) {
        let want = tint.map(|tint| tint.to_srgba_unmultiplied());
        if self.applied == want || (self.unsupported && want.is_some()) {
            return;
        }
        let Some(tint) = want else {
            clear(frame);
            self.applied = None;
            return;
        };
        if let Err(err) = apply(frame, tint) {
            tracing::info!("Glass: the desktop behind the window can't be blurred: {err}");
            self.unsupported = true;
            self.applied = None;
        } else {
            self.applied = Some(tint);
        }
    }
}

#[cfg(windows)]
fn apply(frame: &eframe::Frame, tint: [u8; 4]) -> Result<(), window_vibrancy::Error> {
    window_vibrancy::apply_blur(frame, Some(tint.into()))
}

#[cfg(not(windows))]
fn apply(_frame: &eframe::Frame, _tint: [u8; 4]) -> Result<(), &'static str> {
    Err("only on Windows")
}

#[cfg(windows)]
fn clear(frame: &eframe::Frame) {
    if let Err(err) = window_vibrancy::clear_blur(frame) {
        tracing::warn!("Glass: the blur behind the window stays: {err}");
    }
}

#[cfg(not(windows))]
const fn clear(_frame: &eframe::Frame) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturation_moves_away_from_gray() {
        let mut gray = [128, 128, 128, 255];
        saturate(&mut gray, 1.35);
        assert_eq!(gray, [128, 128, 128, 255], "gray has no color to make richer");

        let mut blue = [60, 90, 160, 200];
        saturate(&mut blue, 1.35);
        assert!(blue[2] > 160 && blue[0] < 60, "{blue:?}");
        assert_eq!(blue[3], 200, "alpha stays");
    }

    #[test]
    fn a_wallpaper_is_blurred_small() {
        let image =
            image::RgbaImage::from_fn(1920, 1080, |x, _| image::Rgba([if x < 960 { 0 } else { 255 }, 0, 0, 255]));
        let blurred = blur_wallpaper(&image);
        assert_eq!(blurred.size, [256, 144]);
        // Black and red meet in the middle: blurred, they blend there.
        let middle = blurred.pixels[72 * 256 + 128];
        assert!(middle.r() > 40 && middle.r() < 220, "{middle:?}");
    }
}
