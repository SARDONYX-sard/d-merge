//! Which mods the table shows and in which order: search filter, column sort, range selection.
//!
//! The table has two modes:
//! - sorted by ascending priority, no filter: the rows are the mod list itself, and they can be
//!   reordered by dragging (a drag sets the priorities from the positions)
//! - anything else: a read-only view, as positions no longer are priorities. Its rows are indices
//!   into the mod list ([`VisibleMods`]), recomputed only when the filter or sort changes

use std::rc::Rc;

use gui_shared::settings::{
    DataMode,
    pages::patch::{mod_item::ModItem, mod_list::SortColumn},
};
use stdx::text::contains_ignore_case;

type ViewState = gui_shared::settings::ViewState;

/// Is the table showing the list as it is stored, so that dragging is safe?
pub(crate) fn is_dnd_allowed(view: &ViewState) -> bool {
    view.filter_text.trim().is_empty() && view.sort_column == SortColumn::Priority && view.sort_asc
}

/// A click on a column header: the same column flips the direction, another starts ascending.
pub(crate) fn toggle_sort(view: &mut ViewState, column: SortColumn) {
    if view.sort_column == column {
        view.sort_asc = !view.sort_asc;
    } else {
        view.sort_column = column;
        view.sort_asc = true;
    }
}

/// The lock button: back to the view that can be dragged.
pub(crate) fn unlock(view: &mut ViewState) {
    view.sort_asc = true;
    view.sort_column = SortColumn::Priority;
    view.filter_text.clear();
}

/// Does `item` match the search text? `None` searches every text column.
fn matches_filter(item: &ModItem, text: &str, column: Option<SortColumn>) -> bool {
    let contains = |value: &str| contains_ignore_case(value, text);
    match column {
        None => {
            contains(&item.id) || contains(&item.name) || contains(item.mod_type.as_lower_str()) || contains(&item.site)
        }
        Some(SortColumn::Id) => contains(&item.id),
        Some(SortColumn::Name) => contains(&item.name),
        Some(SortColumn::ModType) => contains(item.mod_type.as_lower_str()),
        Some(SortColumn::Site) => contains(&item.site),
        Some(SortColumn::Priority) => item.priority.to_string().contains(text),
    }
}

/// Indices into `mods` of the rows to show, filtered and sorted.
fn compute_rows(mods: &[ModItem], view: &ViewState) -> Vec<u32> {
    let text = view.filter_text.trim();
    #[expect(clippy::cast_possible_truncation, reason = "far fewer than 4 billion mods")]
    let mut rows: Vec<u32> = mods
        .iter()
        .enumerate()
        .filter(|(_, item)| text.is_empty() || matches_filter(item, text, view.filter_column))
        .map(|(index, _)| index as u32)
        .collect();
    rows.sort_unstable_by(|&a, &b| {
        let (a, b) = (&mods[a as usize], &mods[b as usize]);
        let order = match view.sort_column {
            SortColumn::Id => a.id.cmp(&b.id),
            SortColumn::Name => a.name.cmp(&b.name),
            SortColumn::ModType => a.mod_type.cmp(&b.mod_type),
            SortColumn::Site => a.site.cmp(&b.site),
            SortColumn::Priority => a.priority.cmp(&b.priority),
        };
        if view.sort_asc { order } else { order.reverse() }
    });
    rows
}

/// What [`VisibleMods::rows`] were computed from.
#[derive(Debug, PartialEq, Eq)]
struct RowsKey {
    mode: DataMode,
    len: usize,
    filter_text: String,
    filter_column: Option<SortColumn>,
    sort_column: SortColumn,
    sort_asc: bool,
}

impl RowsKey {
    fn matches(&self, mode: DataMode, len: usize, view: &ViewState) -> bool {
        self.mode == mode
            && self.len == len
            && self.filter_text == view.filter_text
            && self.filter_column == view.filter_column
            && self.sort_column == view.sort_column
            && self.sort_asc == view.sort_asc
    }
}

/// The rows of the read-only view, kept between frames: filtering lowercases every mod's text.
///
/// Ticking a mod's box does not change them. What reorders the list must call
/// [`Self::invalidate`].
#[derive(Debug, Default)]
pub(crate) struct VisibleMods {
    key: Option<RowsKey>,
    /// An `Rc`: the table borrows the page mutably while it goes through these.
    rows: Option<Rc<[u32]>>,
}

impl VisibleMods {
    pub(crate) fn invalidate(&mut self) {
        self.key = None;
    }

    /// The rows of the read-only view of `mods`.
    pub(crate) fn rows(&mut self, mods: &[ModItem], mode: DataMode, view: &ViewState) -> Rc<[u32]> {
        if let (Some(key), Some(rows)) = (&self.key, &self.rows)
            && key.matches(mode, mods.len(), view)
        {
            return Rc::clone(rows);
        }
        let rows: Rc<[u32]> = compute_rows(mods, view).into();
        self.rows = Some(Rc::clone(&rows));
        self.key = Some(RowsKey {
            mode,
            len: mods.len(),
            filter_text: view.filter_text.clone(),
            filter_column: view.filter_column,
            sort_column: view.sort_column,
            sort_asc: view.sort_asc,
        });
        rows
    }
}

/// Shift+click in the draggable table: every mod between `anchor` and `clicked` gets `enabled`.
pub(crate) fn apply_range(items: &mut [ModItem], anchor: &str, clicked: &str, enabled: bool) {
    let position = |id: &str| items.iter().position(|item| item.id == id);
    let (Some(a), Some(b)) = (position(anchor), position(clicked)) else { return };
    for item in &mut items[a.min(b)..=a.max(b)] {
        item.enabled = enabled;
    }
}

/// Shift+click in the read-only view: the same, by the order on screen (`rows`).
pub(crate) fn apply_visible_range(rows: &[u32], items: &mut [ModItem], anchor: &str, clicked: &str, enabled: bool) {
    let position = |id: &str| rows.iter().position(|&row| items[row as usize].id == id);
    let (Some(a), Some(b)) = (position(anchor), position(clicked)) else { return };
    for &row in &rows[a.min(b)..=a.max(b)] {
        items[row as usize].enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use mod_info::ModType;

    use super::*;

    fn item(id: &str, name: &str, priority: usize) -> ModItem {
        ModItem {
            enabled: false,
            id: id.to_owned(),
            name: name.to_owned(),
            mod_type: ModType::Nemesis,
            site: String::new(),
            priority,
        }
    }

    fn sample() -> Vec<ModItem> {
        vec![item("aaa", "Zebra", 1), item("bbb", "Apple", 2), item("ccc", "Mango", 3)]
    }

    fn view(filter_text: &str, sort_column: SortColumn, sort_asc: bool) -> ViewState {
        ViewState { filter_text: filter_text.to_owned(), filter_column: None, sort_column, sort_asc }
    }

    #[test]
    fn only_the_stored_order_can_be_dragged() {
        assert!(is_dnd_allowed(&view("  ", SortColumn::Priority, true)));
        assert!(!is_dnd_allowed(&view("a", SortColumn::Priority, true)));
        assert!(!is_dnd_allowed(&view("", SortColumn::Priority, false)));
        assert!(!is_dnd_allowed(&view("", SortColumn::Name, true)));
    }

    #[test]
    fn filters_ignoring_case_and_sorts() {
        let mods = sample();
        assert_eq!(compute_rows(&mods, &view("", SortColumn::Name, true)), [1, 2, 0]);
        assert_eq!(compute_rows(&mods, &view("", SortColumn::Name, false)), [0, 2, 1]);
        assert_eq!(compute_rows(&mods, &view("APPLE", SortColumn::Priority, true)), [1]);

        let by_id = ViewState { filter_column: Some(SortColumn::Id), ..view("apple", SortColumn::Priority, true) };
        assert!(compute_rows(&mods, &by_id).is_empty());
    }

    #[test]
    fn rows_are_recomputed_only_when_needed() {
        let mut mods = sample();
        let mut visible = VisibleMods::default();
        let name_view = view("", SortColumn::Name, true);
        assert_eq!(*visible.rows(&mods, DataMode::Vfs, &name_view), [1, 2, 0]);

        mods[0].name = "Aardvark".to_owned(); // not noticed: only `invalidate` says so
        assert_eq!(*visible.rows(&mods, DataMode::Vfs, &name_view), [1, 2, 0]);
        visible.invalidate();
        assert_eq!(*visible.rows(&mods, DataMode::Vfs, &name_view), [0, 1, 2]);

        // Another filter or list does count.
        assert_eq!(*visible.rows(&mods, DataMode::Vfs, &view("mango", SortColumn::Name, true)), [2]);
        mods.pop();
        assert!(visible.rows(&mods, DataMode::Vfs, &view("mango", SortColumn::Name, true)).is_empty());
    }

    #[test]
    fn toggling_a_sort_flips_or_starts_ascending() {
        let mut state = view("", SortColumn::Name, true);
        toggle_sort(&mut state, SortColumn::Name);
        assert!(!state.sort_asc);
        toggle_sort(&mut state, SortColumn::Id);
        assert!(state.sort_asc);
        assert_eq!(state.sort_column, SortColumn::Id);
    }

    #[test]
    fn ranges_work_in_either_direction() {
        let mut mods = sample();
        apply_range(&mut mods, "ccc", "bbb", true);
        assert_eq!(mods.iter().map(|m| m.enabled).collect::<Vec<_>>(), [false, true, true]);

        // On screen: bbb, ccc, aaa. From bbb to aaa covers all three.
        let mut mods = sample();
        apply_visible_range(&[1, 2, 0], &mut mods, "bbb", "aaa", true);
        assert!(mods.iter().all(|m| m.enabled));

        // An anchor that is gone changes nothing.
        let mut mods = sample();
        apply_range(&mut mods, "gone", "aaa", true);
        assert!(!mods[0].enabled);
    }
}
