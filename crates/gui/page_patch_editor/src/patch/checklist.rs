//! A long list of names to check, with a filter: the logic of the two lists of the animation data
//! window ([`super::ui::anim_data`]).
//!
//! The lists are long (a mod can have thousands of hkx, `DefaultMale` has 374 anim set files) and
//! are drawn every frame, so what depends on the whole list is kept up to date instead of being
//! computed again: `visible` changes only with the filter, the count only with a check.

use stdx::text::contains_ignore_case;

/// See the module docs.
#[derive(Debug)]
pub(super) struct Checklist {
    items: Vec<String>,
    checked: Vec<bool>,
    checked_count: usize,
    /// Edited by the view, which then calls [`Self::refilter`].
    pub(super) query: String,
    /// Indices into `items` of those matching `query`.
    visible: Vec<usize>,
}

impl Checklist {
    pub(super) fn new(items: Vec<String>) -> Self {
        let visible = (0..items.len()).collect();
        let checked = vec![false; items.len()];
        Self { items, checked, checked_count: 0, query: String::new(), visible }
    }

    pub(super) const fn len(&self) -> usize {
        self.items.len()
    }

    pub(super) const fn checked_count(&self) -> usize {
        self.checked_count
    }

    /// Indices of the items to show. Use them with [`Self::item`], [`Self::is_checked`], [`Self::set`].
    pub(super) fn visible(&self) -> &[usize] {
        &self.visible
    }

    pub(super) fn item(&self, index: usize) -> &str {
        &self.items[index]
    }

    pub(super) fn is_checked(&self, index: usize) -> bool {
        self.checked[index]
    }

    pub(super) fn set(&mut self, index: usize, on: bool) {
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
    pub(super) fn set_visible(&mut self, on: bool) {
        for at in 0..self.visible.len() {
            self.set(self.visible[at], on);
        }
    }

    /// Call when [`Self::query`] changed.
    pub(super) fn refilter(&mut self) {
        let query = self.query.trim();
        let matching = self.items.iter().enumerate().filter(|(_, item)| contains_ignore_case(item, query));
        self.visible = matching.map(|(index, _)| index).collect();
    }

    /// The checked items, in list order.
    pub(super) fn checked_items(&self) -> impl Iterator<Item = &str> {
        self.items.iter().zip(&self.checked).filter(|(_, on)| **on).map(|(item, _)| item.as_str())
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
