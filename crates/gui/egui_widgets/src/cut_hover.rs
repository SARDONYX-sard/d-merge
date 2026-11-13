//! The whole of what a list's row cuts off at its edge, on hover after the pointer rests:
//!
//! ```text
//! │ #0106 hkbStateMachine GetUpSta│   <- cut off at the side bar's edge
//!          ┌───────────────────────────────┐
//!          │ #0106                         │   <- after 0.7 s still, on lines of its own
//!          │ hkbStateMachine               │
//!          │ GetUpStateMachine             │
//!          └───────────────────────────────┘
//! ```
//! A row seen whole has none. egui shows the next tooltips at once once one was shown: moving over
//! a list would show one per row, so the pointer has to rest first ([`CutHover::delay`]).
//!
//! A row is best cut with `…` ([`fit`]), as VS Code's lists: a list that scrolls sideways changes
//! its width with the rows shown, and its scroll bars come and go.

/// See the module docs. e.g. `CutHover::new().show(ui, response, |ui| { ui.label(full) })`
#[derive(Debug, Clone, Copy)]
#[must_use = "Call `show` to give the response its hover"]
pub struct CutHover {
    delay: f64,
    max_width: f32,
}

impl Default for CutHover {
    fn default() -> Self {
        Self::new()
    }
}

impl CutHover {
    /// Default of [`Self::delay`]: seconds.
    const DELAY: f64 = 0.7;
    /// Default of [`Self::max_width`].
    const MAX_WIDTH: f32 = 420.0;

    #[inline]
    pub const fn new() -> Self {
        Self { delay: Self::DELAY, max_width: Self::MAX_WIDTH }
    }

    /// Seconds the pointer rests on the row before it shows (default 0.7).
    #[inline]
    pub const fn delay(mut self, seconds: f64) -> Self {
        self.delay = seconds;
        self
    }

    /// How wide the hover gets; longer lines wrap (default 420).
    #[inline]
    pub const fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// `response` with `contents` on hover, if what it shows goes past `ui`'s visible right edge.
    pub fn show(self, ui: &egui::Ui, response: egui::Response, contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
        let cut = is_cut(response.rect, ui.clip_rect());
        self.show_if(ui, response, cut, contents)
    }

    /// `response` with `contents` on hover, if `cut` (e.g. [`Fitted::elided`]).
    pub fn show_if(
        self,
        ui: &egui::Ui,
        response: egui::Response,
        cut: bool,
        contents: impl FnOnce(&mut egui::Ui),
    ) -> egui::Response {
        if !response.hovered() || !cut {
            return response;
        }
        let still = f64::from(ui.input(|i| i.pointer.time_since_last_movement()));
        if still < self.delay {
            ui.ctx().request_repaint_after(std::time::Duration::from_secs_f64(self.delay - still));
            return response;
        }
        let max_width = self.max_width;
        response.on_hover_ui(|ui| {
            ui.set_max_width(max_width);
            contents(ui);
        })
    }

    /// [`Self::show`] with `text` wrapped, in the monospace font.
    pub fn show_text(self, ui: &egui::Ui, response: egui::Response, text: &str) -> egui::Response {
        self.show(ui, response, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap());
        })
    }
}

/// A text laid out to fit a width, and whether it had to be cut for it. See [`fit`].
#[derive(Debug, Clone)]
pub struct Fitted {
    pub galley: std::sync::Arc<egui::Galley>,
    /// Cut with `…`: the whole is for a hover ([`CutHover::show_if`]).
    pub elided: bool,
}

/// `job` on one line of at most `width`, cut with `…` if longer, as VS Code's lists.
///
/// e.g. `#0106 hkbStateMachine GetUpStateMachine` in 120 px -> `#0106 hkbState…`, `elided`
#[must_use]
pub fn fit(ui: &egui::Ui, mut job: egui::text::LayoutJob, width: f32) -> Fitted {
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let elided = galley.elided;
    Fitted { galley, elided }
}

/// [`fit`] of a plain text in the monospace font of `color`.
#[must_use]
pub fn fit_text(ui: &egui::Ui, text: &str, color: egui::Color32, width: f32) -> Fitted {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    fit(ui, egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color), width)
}

/// Does `rect` go past the right edge of `visible`? (Half a pixel is not.)
fn is_cut(rect: egui::Rect, visible: egui::Rect) -> bool {
    rect.right() > visible.right() + 0.5
}

/// A selectable button of `text` cut with `…` to the width left, the whole (monospace, wrapped) on a
/// delayed hover when it was: a row of a list that scrolls only up and down, as VS Code's.
pub fn fitted_button(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let width = ui.spacing().button_padding.x.mul_add(-2.0, ui.available_width());
    let fitted = fit_text(ui, text, ui.visuals().text_color(), width);
    let response = ui.add(egui::Button::selectable(selected, fitted.galley));
    CutHover::new().show_if(ui, response, fitted.elided, |ui| {
        ui.add(egui::Label::new(egui::RichText::new(text.trim()).monospace()).wrap());
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_what_goes_past_the_edge_is_cut() {
        let visible = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let row = |right: f32| egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(right, 10.0));
        assert!(is_cut(row(140.0), visible));
        assert!(!is_cut(row(100.3), visible));
        assert!(!is_cut(row(60.0), visible));
    }
}
