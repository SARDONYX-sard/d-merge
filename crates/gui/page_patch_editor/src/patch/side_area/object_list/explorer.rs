//! The list part of the OBJECTS section, as the explorer's own: the object tree, the animation
//! data's list, or the flat list. Each sees only the explorer's state, the graph, this frame's keys
//! and the selection, and returns what the tool must do ([`ListOutput`]).
//!
//! ```text
//! tree   a tree from the root container             (no filter, the tree view)
//! anim   folders > groups > files of animation data
//! flat   one row per object                         (a filter, "unlinked only", or the list view)
//! ```

use std::rc::Rc;

use gui_shared::i18n::{I18nKey, I18nMap};
use stdx::text::{cmp_ignore_case, contains_ignore_case};

use super::{
    RowAction,
    anim_list::{self, AnimEvent, DuplicatePick, Files},
    flat::{self, FlatEvent},
};
use crate::patch::{
    side_area::{
        anim_tree::{self, AnimFile, AnimRow},
        explorer::{Explorer, ObjectList, take_reveal},
    },
    tree::{Graph, Node, ObjectTree, TreeKey},
};

/// What the list asks of the tool.
#[derive(Debug, Default)]
pub(super) struct ListOutput {
    /// `Enter` / `Esc`: the keyboard goes back to the editor.
    pub(super) leave: bool,
    /// A row's menu, or a double click.
    pub(super) menu: Option<(Node, RowAction)>,
    /// Animation data: duplicate these files (of project `from`) to another project.
    pub(super) duplicate: Option<(DuplicatePick, Files, String)>,
}

impl Explorer {
    /// The object tree of the behavior at `path` (its template path: each has its own tree).
    pub(super) fn tree(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (graph, path): (&Graph, &str),
        keys: &[TreeKey],
        selected: &mut Option<Rc<str>>,
    ) -> ListOutput {
        if !self.trees.contains_key(path) {
            self.trees.insert(path.to_owned(), crate::patch::tree::TreeState::default());
        }
        let Some(tree) = self.trees.get_mut(path) else { return ListOutput::default() };
        if std::mem::take(&mut self.reveal) {
            tree.reveal_again();
        }

        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let source = ObjectTree { graph, changed_only: self.changed_only };
            tree.prepare(&source, selected.as_deref());
            let done = tree.apply_keys(&source, keys);
            if let Some(node) = done.selected {
                *selected = Some(graph.id(node).into());
            }
            if done.leave {
                return ListOutput { leave: true, ..ListOutput::default() };
            }
        }

        let flags = (self.changed_only, self.tree_focus);
        let menu = crate::patch::side_area::tree::tree_ui(ui, graph, selected, tree, flags, i18n);
        ListOutput { menu, ..ListOutput::default() }
    }

    /// The animation data's files after the filter: see [`anim_tree::rows`]. `projects`: those files
    /// can be duplicated to.
    pub(super) fn anim(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (graph, is_adsf, projects): (&Graph, bool, &[String]),
        keys: &[TreeKey],
        selected: &mut Option<Rc<str>>,
    ) -> ListOutput {
        self.refresh_list(graph, true);

        // Only when the editor gets the keyboard: else what is open, and where it is scrolled, is the user's.
        let forced = std::mem::take(&mut self.reveal);
        let reveal = take_reveal(selected.as_ref(), forced);
        if reveal && let Some(id) = selected.as_deref() {
            anim_tree::reveal(&mut self.anim_tree.open, is_adsf, id);
        }

        let files: Vec<AnimFile<'_>> = {
            let nodes = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
            nodes.iter().map(|&node| AnimFile { node, id: graph.id(node), changed: graph.is_changed(node) }).collect()
        };
        if std::mem::take(&mut self.anim_tree.expand_all) {
            anim_tree::expand_all(&mut self.anim_tree.open, &files);
        }
        let filtering = !self.filter.is_empty();
        let rebuild = |state: &anim_tree::AnimTreeState| anim_tree::rows(&files, &state.open, is_adsf, filtering);
        let mut rows = rebuild(&self.anim_tree);

        let mut output = ListOutput::default();
        let moved = self.anim_keys(ui, i18n, (graph, keys), (&mut rows, &rebuild), (selected, &mut output));
        let cursor = self.tree_focus.then_some(self.anim_tree.cursor);

        let (event, scroll) = {
            let node = selected.as_deref().and_then(|id| graph.node_of(id));
            let selected_row = node.and_then(|node| file_row(&rows, node));
            let input = anim_list::ListInput {
                graph,
                selected: selected.as_deref(),
                cursor,
                scroll_to: if moved { cursor } else { selected_row.filter(|_| reveal) },
                scroll: self.list_scroll,
                projects,
            };
            anim_list::show(ui, i18n, &rows, &input)
        };
        self.list_scroll = scroll;

        match event {
            Some(AnimEvent::Toggle(key)) => anim_tree::toggle(&mut self.anim_tree.open, &key),
            Some(AnimEvent::ExpandBelow(folder)) => anim_tree::expand(&mut self.anim_tree.open, &folder),
            Some(AnimEvent::CollapseBelow(folder)) => anim_tree::collapse(&mut self.anim_tree.open, &folder),
            Some(AnimEvent::Select(node)) => *selected = Some(graph.id(node).into()),
            Some(AnimEvent::Menu(node, action)) => output.menu = Some((node, action)),
            Some(AnimEvent::Duplicate { pick, files, from }) => output.duplicate = Some((pick, files, from)),
            None => {}
        }
        output
    }

    /// The anim list's keys, while it has the keyboard: a file the cursor gets to is selected. Else
    /// the cursor follows the selection. Returns whether the keys moved it.
    fn anim_keys<'a>(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        (graph, keys): (&Graph, &[TreeKey]),
        (rows, rebuild): (&mut Vec<AnimRow<'a>>, &impl Fn(&anim_tree::AnimTreeState) -> Vec<AnimRow<'a>>),
        (selected, output): (&mut Option<Rc<str>>, &mut ListOutput),
    ) -> bool {
        let mut moved = false;
        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            let done = anim_list::keys(&mut self.anim_tree, keys, rows, rebuild);
            output.leave = done.leave;
            moved = done.moved;
            if moved && let Some(AnimRow::File { node, .. }) = rows.get(self.anim_tree.cursor) {
                *selected = Some(graph.id(*node).into());
            }
        } else if let Some(at) =
            selected.as_deref().and_then(|id| graph.node_of(id)).and_then(|node| file_row(rows, node))
        {
            self.anim_tree.cursor = at; // the keyboard starts where the selection is
        }

        self.anim_tree.cursor = self.anim_tree.cursor.min(rows.len().saturating_sub(1));
        moved
    }

    /// The flat list after the filter (only the rows on screen are laid out).
    pub(super) fn flat(
        &mut self,
        ui: &mut egui::Ui,
        i18n: &I18nMap,
        graph: &Graph,
        keys: &[TreeKey],
        selected: &mut Option<Rc<str>>,
    ) -> ListOutput {
        self.refresh_list(graph, false);

        let mut output = ListOutput::default();
        if self.tree_focus {
            ui.weak(i18n.t(I18nKey::NemesisTreeKeysHint));
            output.leave = self.list_keys(graph, keys, selected);
        }

        let rows = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        // Only when the editor gets the keyboard: else where it is scrolled is the user's.
        let forced = std::mem::take(&mut self.reveal);
        if take_reveal(selected.as_ref(), forced) {
            let node = selected.as_deref().and_then(|id| graph.node_of(id));
            self.list_scroll_to = node.and_then(|node| rows.iter().position(|&row| row == node));
        }

        let scroll = (self.list_scroll_to.take(), self.list_scroll);
        let (event, scroll) = flat::show(ui, i18n, (graph, rows), selected.as_deref(), scroll);
        self.list_scroll = scroll;

        match event {
            Some(FlatEvent::Select(node)) => *selected = Some(graph.id(node).into()),
            Some(FlatEvent::Menu(node, action)) => output.menu = Some((node, action)),
            None => {}
        }
        output
    }

    /// The flat list's keys: the same as the tree's, without the folding. Returns whether the
    /// keyboard goes back to the editor.
    fn list_keys(&mut self, graph: &Graph, keys: &[TreeKey], selected: &mut Option<Rc<str>>) -> bool {
        let rows = self.list.as_ref().map_or(&[][..], |list| list.rows.as_slice());
        let before = rows.iter().position(|&node| selected.as_deref() == Some(graph.id(node))).unwrap_or(0);

        let mut current = before;
        let mut leave = false;
        for &key in keys {
            leave |= key == TreeKey::Leave;
            current = key.step(current, rows.len()).unwrap_or(current);
        }

        if let Some(&node) = rows.get(current) {
            *selected = Some(graph.id(node).into());
        }
        if current != before {
            self.list_scroll_to = Some(current);
        }
        leave
    }

    /// The flat list's rows again, if the graph, the filter or the toggles changed since. Animation
    /// data is sorted by id: grouped by actor (see `ui::anim_tree`).
    fn refresh_list(&mut self, graph: &Graph, is_anim_data: bool) {
        let generation = if self.filter.is_empty() { graph.tree_generation() } else { graph.generation() };
        let fresh = self.list.as_ref().is_some_and(|list| {
            list.generation == generation
                && list.filter == self.filter
                && list.changed_only == self.changed_only
                && list.unlinked_only == self.unlinked_only
        });
        if fresh {
            return;
        }

        let filter = self.filter.as_str();
        let mut rows: Vec<Node> = graph
            .nodes()
            .filter(|&node| !self.changed_only || graph.is_changed(node))
            .filter(|&node| !self.unlinked_only || graph.is_unlinked(node))
            .filter(|&node| contains_ignore_case(graph.label(node), filter))
            .collect();
        if is_anim_data {
            rows.sort_by(|&a, &b| cmp_ignore_case(graph.id(a), graph.id(b)));
        }

        self.list = Some(ObjectList {
            generation,
            filter: self.filter.clone(),
            changed_only: self.changed_only,
            unlinked_only: self.unlinked_only,
            rows,
        });
    }
}

/// The row of file `node`, if it is shown.
fn file_row(rows: &[AnimRow<'_>], node: Node) -> Option<usize> {
    rows.iter().position(|row| matches!(row, AnimRow::File { node: n, .. } if *n == node))
}
