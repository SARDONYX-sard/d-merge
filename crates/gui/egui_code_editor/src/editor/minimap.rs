//! The minimap, as VS Code's: the whole text shrunk at the right of the view, in its colors, with
//! the part in view under a slider. A click or a drag on it scrolls the text there.
//!
//! ```text
//! ┌ view ─────────────────────────────┬ minimap ┐
//! │ <hkobject name="#0106" ...>       │ ▬▬ ▬▬▬  │
//! │   <hkparam name="id">#0105</...>  │░▬ ▬▬▬░░░│ <- the slider: the lines in view
//! │   ...                             │░▬▬ ▬ ░░░│
//! │                                   │ ▬▬▬ ▬   │
//! └───────────────────────────────────┴─────────┘
//! ```
//!
//! Each row is [`ROW`] points tall, its glyphs shrunk as much, a run of one color a bar. A text
//! taller than the view in the minimap scrolls in it, as the text does: at the top together, at the
//! bottom together. Only the rows in it are drawn.

use egui::{Galley, Pos2, Rect, Sense};

/// The height of a row in the minimap, in points.
const ROW: f32 = 2.0;
/// The share of a row's height its bars take: a gap between rows.
const BAR: f32 = 0.75;

/// The bars of each row of a layout: read once per layout (the glyphs do not tell their color, the
/// layout's sections do, along its text).
#[derive(Debug, Default)]
pub(crate) struct Bars {
    /// The layout they are of, by the address of its job: another one, they are read again. Not the
    /// galley's address: egui copies the galley (not its job) to paint a selection, every frame
    /// there is one.
    of: usize,
    rows: Vec<Vec<(f32, f32, egui::Color32)>>,
}

impl Bars {
    /// The bars of `galley`: of each row, each run of glyphs of one color `(left, right, color)`,
    /// whitespace left out.
    fn of(galley: &Galley) -> Self {
        let sections = &galley.job.sections;
        let mut chars = galley.job.text.char_indices();
        let mut section = 0;
        let mut color_at = |byte: usize| {
            while sections.get(section).is_some_and(|it| it.byte_range.end <= egui::text::ByteIndex(byte)) {
                section += 1;
            }
            sections.get(section).map(|it| it.format.color)
        };
        let rows = galley
            .rows
            .iter()
            .map(|row| {
                let mut bars: Vec<(f32, f32, egui::Color32)> = Vec::new();
                let mut joined = false;
                for glyph in &row.glyphs {
                    let color = chars.next().and_then(|(byte, _)| color_at(byte));
                    let (left, right) = (glyph.pos.x, glyph.pos.x + glyph.advance_width);
                    match (color, bars.last_mut()) {
                        _ if glyph.chr.is_whitespace() => joined = false,
                        (Some(color), Some(last)) if joined && last.2 == color => last.1 = right,
                        (Some(color), _) => {
                            bars.push((left, right, color));
                            joined = true;
                        }
                        (None, _) => joined = false,
                    }
                }
                if row.ends_with_newline {
                    chars.next();
                }
                bars
            })
            .collect();
        Self { of: layout_of(galley), rows }
    }
}

/// What [`Bars::of`] says which layout it is of. See there.
fn layout_of(galley: &Galley) -> usize {
    std::sync::Arc::as_ptr(&galley.job) as usize
}

/// Draws the minimap at the right of `view` (the editor's part on screen), `width` wide, for the
/// text laid out in `galley` at `galley_pos`. A press or a drag on it scrolls the text there.
/// `bars`: kept from one frame to the next.
pub(crate) fn show(
    ui: &egui::Ui,
    (id, bars): (egui::Id, &mut Option<Bars>),
    (galley, galley_pos): (&Galley, Pos2),
    (view, width): (Rect, f32),
) {
    let Some(row_height) = galley.rows.first().map(|row| row.rect().height()).filter(|height| *height > 0.0) else {
        return;
    };
    let scale = ROW / row_height;
    let rect = Rect::from_min_max(egui::pos2(view.right() - width, view.top()), view.max);
    let map = Map::new(galley.rect.height(), view.height(), view.top() - galley_pos.y, scale);

    let response = ui.interact(rect, id.with("minimap"), Sense::click_and_drag());
    if let Some(pointer) = response.interact_pointer_pos().filter(|_| response.is_pointer_button_down_on()) {
        // The slider's middle at the pointer, at once: the minimap scrolls too as the text does, so
        // the place is found for both, not through where the minimap is now.
        let top = galley_pos.y + map.scrolled_for(pointer.y - rect.top());
        let target = Rect::from_x_y_ranges(galley_pos.x..=galley_pos.x + 1.0, top..=top + 1.0);
        ui.scroll_to_rect_animation(target, Some(egui::Align::Min), super::decorations::instant());
    }

    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color.gamma_multiply(0.6));
    if bars.as_ref().is_none_or(|bars| bars.of != layout_of(galley)) {
        *bars = Some(Bars::of(galley));
    }
    let rows = bars.as_ref().map_or(&[][..], |bars| &bars.rows);
    for (row, row_bars) in galley.rows.iter().zip(rows) {
        let top = rect.top() + map.mini_y(row.pos.y);
        if top + ROW < rect.top() || top > rect.bottom() {
            continue;
        }
        let left = rect.left() + 4.0;
        for &(start, end, color) in row_bars {
            let bar = Rect::from_min_max(
                egui::pos2(start.mul_add(scale, left), top),
                egui::pos2(end.mul_add(scale, left), ROW.mul_add(BAR, top)),
            );
            painter.rect_filled(bar.intersect(rect), 0.0, color.gamma_multiply(0.8));
        }
    }

    let slider = Rect::from_x_y_ranges(rect.x_range(), {
        let top = rect.top() + map.mini_y(map.scrolled);
        top..=view.height().mul_add(scale, top)
    });
    let strength = if response.hovered() || response.dragged() { 0.35 } else { 0.18 };
    painter.rect_filled(slider, 0.0, visuals.text_color().gamma_multiply(strength * 0.5));
}

/// Where the text is in the minimap, and the other way round. See the module docs.
#[derive(Debug, Clone, Copy)]
struct Map {
    /// How far the text is scrolled: the top of the view in the text.
    scrolled: f32,
    scale: f32,
    /// How far the minimap is scrolled, in its points.
    offset: f32,
    /// How far the text can be scrolled, and how far the minimap then.
    room: f32,
    excess: f32,
    view_height: f32,
}

impl Map {
    /// `text_height` tall in a view `view_height` tall, scrolled `scrolled`.
    ///
    /// e.g. a text of 1000 rows of 20 (20000 tall) in a view of 500, scrolled to its end (19500): the
    /// minimap is 2000 tall, scrolled 1500 (to its end too).
    fn new(text_height: f32, view_height: f32, scrolled: f32, scale: f32) -> Self {
        let mini_height = text_height * scale;
        let room = (text_height - view_height).max(1.0);
        let fraction = (scrolled / room).clamp(0.0, 1.0);
        let excess = (mini_height - view_height).max(0.0);
        Self { scrolled: scrolled.max(0.0), scale, offset: excess * fraction, room, excess, view_height }
    }

    /// How far the text is to be scrolled for the slider's middle to be at `y` of the minimap. The
    /// slider's top is at `scrolled * (scale - excess / room)`: the text scrolls, and the minimap
    /// with it.
    fn scrolled_for(self, y: f32) -> f32 {
        let along = (self.scale - self.excess / self.room).max(f32::EPSILON);
        let half_slider = self.view_height * self.scale / 2.0;
        ((y - half_slider) / along).clamp(0.0, self.room)
    }

    /// The top of text `y` in the minimap, from its top on screen.
    fn mini_y(self, y: f32) -> f32 {
        y.mul_add(self.scale, -self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bar per run of one color, the whitespace between left out; read along the layout's
    /// sections, row by row.
    #[test]
    fn bars_follow_the_colors() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = egui::FontId::monospace(10.0);
            let mut job = egui::text::LayoutJob::default();
            let (red, blue) = (egui::Color32::RED, egui::Color32::BLUE);
            job.append("ab c", 0.0, egui::TextFormat::simple(font.clone(), red));
            job.append(
                "d
ef",
                0.0,
                egui::TextFormat::simple(font, blue),
            );
            let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
            let bars = Bars::of(&galley);
            let colors: Vec<Vec<egui::Color32>> =
                bars.rows.iter().map(|row| row.iter().map(|bar| bar.2).collect()).collect();
            assert_eq!(colors, [vec![red, red, blue], vec![blue]], "`ab` `c` `d` / `ef`");

            // What egui does to paint a selection: the same layout, not read again.
            let copied = (*galley).clone();
            assert_eq!(layout_of(&copied), bars.of, "a copy of the galley");
        });
        output.textures_delta.clear();
    }

    /// A long text scrolls in the minimap with the text: at the top and at the end together; a
    /// point of it is the same text either way.
    #[test]
    fn the_minimap_scrolls_with_the_text() {
        let scale = ROW / 20.0;
        let at_top = Map::new(20_000.0, 500.0, 0.0, scale);
        assert!(at_top.offset.abs() < f32::EPSILON);
        let at_end = Map::new(20_000.0, 500.0, 19_500.0, scale);
        assert!((at_end.offset - 1500.0).abs() < 0.01, "{}", at_end.offset);

        // The slider put where it is: the text stays.
        let middle = Map::new(20_000.0, 500.0, 9_000.0, scale);
        let slider_middle = middle.mini_y(middle.scrolled) + 500.0 * scale / 2.0;
        assert!((middle.scrolled_for(slider_middle) - 9_000.0).abs() < 0.5, "{}", middle.scrolled_for(slider_middle));

        let short = Map::new(400.0, 500.0, 0.0, scale);
        assert!(short.offset.abs() < f32::EPSILON, "shorter than the view: it does not scroll");
    }
}
