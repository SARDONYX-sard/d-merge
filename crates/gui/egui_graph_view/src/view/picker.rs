//! The nodes a pin can be connected to, listed where it was dropped on no node: as a blueprint's
//! menu on a wire let go in the void. The node to connect is often not shown yet (a new one, or one
//! far away), so it is found here by its name.
//!
//! ```text
//!   ●────────╮
//!            ▼
//!   ┌──────────────────────────────┐
//!   │ [Connect to…  idle         ] │  typed: the list narrows (title or subtitle)
//!   │ ✂ Disconnect                 │  only if the pin is connected
//!   │ ▶ Idle   #0097 hkbClipGene…  │  ↑ ↓ select, Enter or a click connects
//!   │   IdleBreak   #0130 hkbCl…   │
//!   └──────────────────────────────┘  Esc or a click beside it: nothing
//! ```

use egui::{Key, Modifiers, Pos2};

use crate::{GraphEdit, GraphSource};

/// The list's width.
const WIDTH: f32 = 340.0;
/// The list's height at most: longer, it scrolls.
const MAX_HEIGHT: f32 = 280.0;

/// The open list. See the module docs.
#[derive(Debug)]
pub(super) struct Picker<N> {
    /// The pin: `(node, row key)`.
    pin: (N, usize),
    /// Where it was dropped, on screen.
    at: Pos2,
    /// The nodes the pin can take, read once when it opens (asking each is not free).
    candidates: Vec<N>,
    /// The pin points somewhere: `Disconnect` is offered.
    connected: bool,
    search: String,
    /// In the narrowed list.
    selected: usize,
    /// The search field got the keyboard: it is asked for once.
    focused: bool,
    /// The selection moved by a key: the list scrolls to it once (the wheel scrolls it else).
    follow: bool,
}

/// What the list asked for.
#[derive(Debug)]
pub(super) enum Picked<N> {
    /// Still open.
    Open,
    /// Closed, with this to do, or nothing.
    Closed(Option<GraphEdit<N>>),
}

impl<N: Copy + Eq + std::hash::Hash + std::fmt::Debug> Picker<N> {
    /// Opens the list of the nodes `pin` can be connected to, at `at`: not itself, nor one only
    /// mentioned ([`crate::Card::dimmed`]), and only those [`GraphSource::accepts`] takes.
    pub(super) fn open<S: GraphSource<Node = N>>(source: &S, pin: (N, usize), at: Pos2, connected: bool) -> Self {
        let (node, key) = pin;
        let candidates = source
            .nodes()
            .filter(|&to| to != node && !source.card(to).dimmed && source.accepts(node, key, to))
            .collect();
        Self { pin, at, candidates, connected, search: String::new(), selected: 0, focused: false, follow: false }
    }

    /// Draws the list. See [`Picked`].
    pub(super) fn ui<S: GraphSource<Node = N>>(
        &mut self,
        ctx: &egui::Context,
        source: &S,
        (hint, disconnect): (&str, &str),
    ) -> Picked<N> {
        let shown = self.narrowed(source);
        let keys = self.keys(ctx, shown.len());
        let area =
            egui::Area::new(egui::Id::NULL.with("graph_view_picker")).order(egui::Order::Foreground).fixed_pos(self.at);
        let mut clicked = None;
        let response = area.show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(WIDTH);
                let field =
                    ui.add(egui::TextEdit::singleline(&mut self.search).hint_text(hint).desired_width(f32::INFINITY));
                if !std::mem::replace(&mut self.focused, true) {
                    field.request_focus();
                }
                if field.changed() {
                    self.selected = 0;
                }
                if self.connected && ui.button(format!("✂ {disconnect}")).clicked() {
                    clicked = Some(None);
                }
                clicked = clicked.or_else(|| self.list_ui(ui, source, &shown).map(Some));
            });
        });

        let pin = self.pin;
        let edit = |to: Option<N>| match to {
            Some(to) => GraphEdit::Connect { node: pin.0, key: pin.1, to },
            None => GraphEdit::Disconnect { node: pin.0, key: pin.1 },
        };
        match (keys, clicked) {
            (_, Some(to)) => Picked::Closed(Some(edit(to))),
            (Some(Key::Enter), None) => Picked::Closed(shown.get(self.selected).map(|&to| edit(Some(to)))),
            (Some(_), None) => Picked::Closed(None),
            (None, None) if response.response.clicked_elsewhere() => Picked::Closed(None),
            (None, None) => Picked::Open,
        }
    }

    /// The candidates whose title or subtitle has the search in it, any case.
    fn narrowed<S: GraphSource<Node = N>>(&self, source: &S) -> Vec<N> {
        let search = self.search.to_lowercase();
        let has = |text: &str| text.to_lowercase().contains(&search);
        let matches = |&to: &N| {
            let card = source.card(to);
            has(card.title) || has(card.subtitle)
        };
        self.candidates.iter().copied().filter(|to| search.is_empty() || matches(to)).collect()
    }

    /// `↑` `↓` select (taken before the field moves its cursor with them). Returns `Enter` or `Esc`,
    /// which close it.
    fn keys(&mut self, ctx: &egui::Context, len: usize) -> Option<Key> {
        ctx.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                self.selected = (self.selected + 1).min(len.saturating_sub(1));
                self.follow = true;
            }
            if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                self.selected = self.selected.saturating_sub(1);
                self.follow = true;
            }
            [Key::Enter, Key::Escape].into_iter().find(|&key| input.consume_key(Modifiers::NONE, key))
        })
    }

    /// `title  subtitle` of each shown candidate, the selected one marked. Returns the one clicked.
    fn list_ui<S: GraphSource<Node = N>>(&mut self, ui: &mut egui::Ui, source: &S, shown: &[N]) -> Option<N> {
        if shown.is_empty() {
            ui.weak("—");
            return None;
        }
        let height = ui.spacing().button_padding.y.mul_add(2.0, ui.text_style_height(&egui::TextStyle::Button));
        let mut scroll = egui::ScrollArea::vertical().max_height(MAX_HEIGHT).auto_shrink([false, true]);
        if std::mem::take(&mut self.follow) {
            // Only the rows in view are drawn: the one selected by a key is scrolled to by its place,
            // in the middle.
            #[expect(clippy::cast_precision_loss, reason = "a list of nodes")]
            let top = self.selected as f32 * (height + ui.spacing().item_spacing.y);
            scroll = scroll.vertical_scroll_offset((top - MAX_HEIGHT / 2.0).max(0.0));
        }
        let mut clicked = None;
        scroll.show_rows(ui, height, shown.len(), |ui, rows| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                for at in rows {
                    let card = source.card(shown[at]);
                    let text = format!("{}   {}", card.title, card.subtitle);
                    let response = ui.add(egui::Button::selectable(at == self.selected, text).truncate());
                    if response.clicked() {
                        clicked = Some(shown[at]);
                    }
                }
            });
        });
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Card, Edge};

    /// Nodes `0..4` titled `n0`..`n3`; `3` is only mentioned; a pin takes odd nodes only.
    struct Mock;

    impl GraphSource for Mock {
        type Node = u32;

        fn version(&self) -> u64 {
            0
        }

        fn edges(&self, _: u32) -> impl Iterator<Item = Edge<'_, u32>> {
            std::iter::empty()
        }

        fn parents(&self, _: u32) -> impl Iterator<Item = u32> {
            std::iter::empty()
        }

        fn nodes(&self) -> impl Iterator<Item = u32> {
            0..4
        }

        fn card(&self, node: u32) -> Card<'_> {
            let title = ["n0", "n1", "n2", "n3"][node as usize];
            Card { title, subtitle: "", header: None, accent: None, dimmed: node == 3, editable: true, fan_in: 0 }
        }

        fn accepts(&self, _: u32, _: usize, to: u32) -> bool {
            to % 2 == 1
        }
    }

    /// Not the pin's own node, not one only mentioned, and only what the pin takes; narrowed by
    /// the search, any case.
    #[test]
    fn the_list_has_what_the_pin_can_take() {
        let mut picker = Picker::open(&Mock, (1, 0), Pos2::ZERO, false);
        assert_eq!(picker.candidates, Vec::<u32>::new(), "1 is itself, 3 only mentioned");

        picker = Picker::open(&Mock, (0, 0), Pos2::ZERO, false);
        assert_eq!(picker.candidates, [1]);
        picker.search = "N1".to_owned();
        assert_eq!(picker.narrowed(&Mock), [1]);
        picker.search = "n2".to_owned();
        assert_eq!(picker.narrowed(&Mock), Vec::<u32>::new());
    }
}
