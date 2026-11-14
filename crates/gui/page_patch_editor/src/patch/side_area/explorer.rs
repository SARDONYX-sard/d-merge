//! The state of the explorer's objects: the filters, the trees and the list, where the keyboard is.

use std::rc::Rc;

pub(super) use egui_tree::list::{ListScroll, fixed_row, offset_to_show, row_height};

/// The state of the objects in the explorer.
///
/// ```text
/// [filter.......]  ⋯ changed only / unlinked only    [≡] tree_view
/// tree (trees, by behavior) or flat list (list)       tree_focus: the keyboard is in it
/// ```
#[derive(Debug, Default)]
pub(super) struct Explorer {
    pub(super) filter: String,
    /// Only the objects changed, and the way to them in the tree.
    pub(super) changed_only: bool,
    /// Only the new objects nothing links to. See [`crate::patch::tree::Graph::unlinked`].
    pub(super) unlinked_only: bool,
    /// A tree from the root container, else a flat list.
    pub(super) tree_view: bool,
    /// The tree of each behavior, by template path.
    pub(super) trees: fast_hash::HashMap<String, crate::patch::tree::TreeState>,
    /// The flat list, as it was computed. See `Explorer::refresh_list`.
    pub(super) list: Option<ObjectList>,

    /// Where the flat list (or the animation data's tree) was scrolled. See [`offset_to_show`].
    pub(super) list_scroll: ListScroll,
    /// The row of the flat list to scroll to: the keys moved to it, or the editor got the keyboard.
    pub(super) list_scroll_to: Option<usize>,
    /// The keyboard is in the tree or the list (Vim `Space e`).
    pub(super) tree_focus: bool,
    /// Show the selected object again, even if it was shown already: the editor got the keyboard.
    pub(super) reveal: bool,
    /// What is open in the animation data's list, and its cursor.
    pub(super) anim_tree: super::anim_tree::AnimTreeState,
}

/// The flat list as it was computed: see [`Explorer::list`].
#[derive(Debug)]
pub(super) struct ObjectList {
    /// The state of the graph it was computed from.
    pub(super) generation: u64,
    pub(super) filter: String,
    pub(super) changed_only: bool,
    pub(super) unlinked_only: bool,
    pub(super) rows: Vec<crate::patch::tree::Node>,
}

/// Should a list scroll to the selection? Only when `forced`: the editor got the keyboard, as VS Code's
/// explorer follows the file being edited. A selection alone moves nothing, or a list being used would.
pub(super) const fn take_reveal(selected: Option<&Rc<str>>, forced: bool) -> bool {
    forced && selected.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_scrolls_only_when_the_editor_gets_the_keyboard() {
        let a: Rc<str> = "#a".into();
        assert!(!take_reveal(Some(&a), false), "a selection alone");
        assert!(take_reveal(Some(&a), true), "the editor got the keyboard");
        assert!(!take_reveal(None, true), "nothing to show");
    }
}
