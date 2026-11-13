//! A long list of names to check, with a filter, drawn only where it is seen:
//!
//! ```text
//! hkx  1 / 12  [All] [None]          <- `All` / `None`: only what the filter shows
//! [filter............................]
//! ☑ Idle.hkx
//! ☐ ParaGlideEquip.hkx               <- a click on the name checks it too
//! ```
//! Lists can be long (a mod can have thousands of files) and are drawn every frame, so what depends on
//! the whole list is kept up to date instead of being computed again: what the filter shows changes
//! only with the filter, the count only with a check.
//!
//! ```text
//! let mut list = Checklist::new(names);                 // kept by the host
//! list.show(ui, "hkx", &labels, 220.0);
//! let picked: Vec<&str> = list.checked_items().collect();
//! ```

use stdx::text::contains_ignore_case;

/// The words of [`Checklist::show`], in the host's language. e.g. `{ all: "All", none: "None",
/// filter: "Filter" }`
#[derive(Debug, Clone, Copy)]
pub struct ChecklistLabels<'a> {
    pub all: &'a str,
    pub none: &'a str,
    /// The filter's hint.
    pub filter: &'a str,
}

/// See the module docs.
#[derive(Debug)]
pub struct Checklist {
    items: Vec<String>,
    checked: Vec<bool>,
    checked_count: usize,
    /// The filter; call [`Self::refilter`] after changing it.
    pub query: String,
    /// Indices into `items` of those matching `query`.
    visible: Vec<usize>,
    row_height: f32,
}

impl Checklist {
    /// Default of [`Self::row_height`].
    const ROW_HEIGHT: f32 = 20.0;

    /// `items`, none checked.
    #[must_use]
    pub fn new(items: Vec<String>) -> Self {
        let visible = (0..items.len()).collect();
        let checked = vec![false; items.len()];
        let row_height = Self::ROW_HEIGHT;
        Self { items, checked, checked_count: 0, query: String::new(), visible, row_height }
    }

    /// The height of a row (default 20).
    #[inline]
    #[must_use]
    pub const fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    #[must_use]
    pub const fn checked_count(&self) -> usize {
        self.checked_count
    }

    /// Indices of the items the filter shows. Use them with [`Self::item`], [`Self::is_checked`],
    /// [`Self::set`].
    #[inline]
    #[must_use]
    pub fn visible(&self) -> &[usize] {
        &self.visible
    }

    #[inline]
    #[must_use]
    pub fn item(&self, index: usize) -> &str {
        &self.items[index]
    }

    #[inline]
    #[must_use]
    pub fn is_checked(&self, index: usize) -> bool {
        self.checked[index]
    }

    pub fn set(&mut self, index: usize, on: bool) {
        if self.checked[index] != on {
            self.checked[index] = on;
            if on {
                self.checked_count += 1;
            } else {
                self.checked_count -= 1;
            }
        }
    }

    /// Checks or unchecks what the filter shows, and only that.
    pub fn set_visible(&mut self, on: bool) {
        for at in 0..self.visible.len() {
            self.set(self.visible[at], on);
        }
    }

    /// Call when [`Self::query`] changed. It ignores ASCII case and the spaces around it.
    pub fn refilter(&mut self) {
        let query = self.query.trim();
        let matching = self.items.iter().enumerate().filter(|(_, item)| contains_ignore_case(item, query));
        self.visible = matching.map(|(index, _)| index).collect();
    }

    /// The checked items, in list order.
    pub fn checked_items(&self) -> impl Iterator<Item = &str> {
        self.items.iter().zip(&self.checked).filter(|(_, on)| **on).map(|(item, _)| item.as_str())
    }

    /// Draws `title  1 / 12  [All] [None]`, the filter, and the rows seen, at most `max_height` high.
    pub fn show(&mut self, ui: &mut egui::Ui, title: &str, labels: &ChecklistLabels<'_>, max_height: f32) {
        ui.push_id(title, |ui| {
            ui.horizontal(|ui| {
                ui.strong(title);
                ui.weak(format!("{} / {}", self.checked_count, self.len()));
                if ui.small_button(labels.all).clicked() {
                    self.set_visible(true);
                }
                if ui.small_button(labels.none).clicked() {
                    self.set_visible(false);
                }
            });
            let filter = egui::TextEdit::singleline(&mut self.query).hint_text(labels.filter);
            if ui.add(filter.desired_width(f32::INFINITY)).changed() {
                self.refilter();
            }
            let scroll = egui::ScrollArea::both().max_height(max_height).auto_shrink([false, false]);
            scroll.show_rows(ui, self.row_height, self.visible.len(), |ui, range| {
                for at in range {
                    self.row(ui, self.visible[at]);
                }
            });
        });
    }

    /// One row. The name can be clicked too; a long one is cut, and is whole on hover.
    fn row(&mut self, ui: &mut egui::Ui, index: usize) {
        ui.horizontal(|ui| {
            ui.set_height(self.row_height);
            let mut on = self.is_checked(index);
            let box_clicked = ui.add(egui::Checkbox::without_text(&mut on)).changed();
            let label = egui::Label::new(self.item(index)).truncate().sense(egui::Sense::click());
            if ui.add(label).clicked() {
                on = !on;
            }
            if box_clicked || on != self.is_checked(index) {
                self.set(index, on);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Checklist {
        Checklist::new(["Idle.hkx", "ParaGlideEquip.hkx", "Run.hkx"].map(str::to_owned).to_vec())
    }

    #[test]
    fn filter_ignores_case_and_keeps_the_checks() {
        let mut list = list();
        list.set(0, true);
        list.query = " paraglide ".to_owned();
        list.refilter();
        assert_eq!(list.visible(), [1]);
        assert_eq!(list.checked_items().collect::<Vec<_>>(), ["Idle.hkx"]);

        list.query.clear();
        list.refilter();
        assert_eq!(list.visible(), [0, 1, 2]);
    }

    #[test]
    fn checking_what_is_visible_leaves_the_rest() {
        let mut list = list();
        list.query = "run".to_owned();
        list.refilter();
        list.set_visible(true);
        assert_eq!(list.checked_items().collect::<Vec<_>>(), ["Run.hkx"]);
        assert_eq!(list.checked_count(), 1);

        list.set_visible(true); // already on: not counted twice
        assert_eq!(list.checked_count(), 1);
        list.set_visible(false);
        assert_eq!(list.checked_count(), 0);
    }
}
