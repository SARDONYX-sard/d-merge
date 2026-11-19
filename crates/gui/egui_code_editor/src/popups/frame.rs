//! The editor's popups as VS Code draws its widgets (a hover, the suggestions, find, the status
//! bar's settings): in the theme's window color, with a thin border, small corners and a light
//! shadow; and a popup opened by a button attached to it.
//!
//! ```text
//! ┌───────────────────────┐   <- [`widget_frame`]: the theme's colors
//! │ ☑ Completion          │
//! │ ☐ Inlay hints         │
//! └───────────────────────┘
//!                       [⚙]   <- [`AnchoredPopup`]: above the button (below if there is no room)
//! ```
//!
//! An `egui::Window` remembers where it was by its title: a page's popup opened where another page's
//! with the same title was left. An [`AnchoredPopup`] is always at its button.

use egui::{Color32, Frame, Shadow, Style};

/// Default of the corners' radius of [`widget_frame`].
const CORNER_RADIUS: u8 = 4;
/// Default of the room inside [`widget_frame`].
const MARGIN: i8 = 6;
/// Default of the shadow of [`widget_frame`]: light and close, as VS Code's widgets.
const SHADOW: Shadow = Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(60) };

/// The frame of a small popup, in the theme's colors: its window color, and its border. Its margin,
/// corners and shadow can be changed with `Frame`'s own methods.
#[inline]
pub fn widget_frame(style: &Style) -> Frame {
    Frame::new()
        .fill(style.visuals.window_fill)
        .stroke(style.visuals.widgets.noninteractive.bg_stroke)
        .corner_radius(CORNER_RADIUS)
        .inner_margin(MARGIN)
        .shadow(SHADOW)
}

/// A popup attached to `anchor`, the button that opens it: above it, else below it; in a
/// [`widget_frame`]. A click beside it or `Esc` closes it (`open` is then `false`).
///
/// e.g. `if let Some(gear) = &status.options { AnchoredPopup::new(gear, &mut open).show(|ui| ...); }`
#[derive(Debug)]
#[must_use = "Call `show` to draw it"]
pub struct AnchoredPopup<'a> {
    anchor: &'a egui::Response,
    open: &'a mut bool,
}

impl<'a> AnchoredPopup<'a> {
    #[inline]
    pub const fn new(anchor: &'a egui::Response, open: &'a mut bool) -> Self {
        Self { anchor, open }
    }

    /// Draws it while `open`. Returns what `add_contents` returned, if it was drawn.
    pub fn show<R>(self, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
        // Above, at the button's end: where a status bar's item opens its settings.
        const WHERE: [egui::RectAlign; 3] =
            [egui::RectAlign::BOTTOM_END, egui::RectAlign::TOP_START, egui::RectAlign::BOTTOM_START];

        let frame = widget_frame(&self.anchor.ctx.global_style());
        let popup = egui::Popup::from_response(self.anchor)
            .open_bool(self.open)
            .align(egui::RectAlign::TOP_END)
            .align_alternatives(&WHERE)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .frame(frame);
        popup.show(add_contents).map(|shown| shown.inner)
    }
}
