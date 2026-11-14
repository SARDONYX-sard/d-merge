//! The object tree in the side bar. What the tree is (its rows, what is open) is
//! [`crate::patch::tree`]; this only draws it, one part per file:
//!
//! ```text
//! ⚠ Not linked (1)                       unlinked.rs   new objects nothing points to
//!   + #new1  hkbClipGenerator
//! ◎ … › Root › GetUpModList               breadcrumb.rs the way to the selected object
//! ▼ #0091 hkbBehaviorGraph  (pinned)      sticky.rs     the parents of what is scrolled to
//! │ ├ #0094 hkbBehaviorGraphData          rows.rs       the rows, their lines and menus
//! │ └ ▶ #0095 hkbStateMachine  Root
//! ```
//! Each part returns what a click asked for ([`TreeEvent`]); [`tree_ui`] applies it once, at the end.

mod breadcrumb;
mod label;
mod rows;
mod sticky;
mod unlinked;

use std::rc::Rc;

use gui_shared::i18n::I18nMap;

pub(super) use self::label::object_button;
use super::object_list::RowAction;
use crate::patch::tree::{Graph, Node, ObjectTree, TreeState};

/// Pixels of indent per tree level.
const INDENT: f32 = 12.0;

/// What a click in the tree asked for.
#[derive(Debug, Clone, Copy)]
enum TreeEvent {
    /// Row `index` was clicked: its object is selected.
    Select(usize),
    /// `▶` / `▼`: that node opens or closes.
    SetOpen(Node, bool),
    /// The menu's `Expand / Collapse All Below`.
    SetOpenBelow(Node, bool),
    /// `◎`: the selected row is scrolled to.
    Locate,
    /// A double click (`KeepOpen`) or the menu of row `index`: see [`RowAction`].
    Menu(usize, RowAction),
}

/// Draws the tree. A click selects an object (`selected`).
///
/// - `changed_only`: only the branches leading to edited / new objects.
/// - `focused`: the keyboard is in the tree: the cursor's row gets a frame.
///
/// Returns what a row's menu (or a double click: keep it open) asked for, of the row's object.
pub(super) fn tree_ui(
    ui: &mut egui::Ui,
    graph: &Graph,
    selected: &mut Option<Rc<str>>,
    state: &mut TreeState,
    (changed_only, focused): (bool, bool),
    i18n: &I18nMap,
) -> Option<(Node, RowAction)> {
    let source = ObjectTree { graph, changed_only };
    state.prepare(&source, selected.as_deref());

    unlinked::show(ui, graph, selected, i18n);

    // The later a part is drawn, the more it wins: the pinned rows are over the others.
    let crumb = breadcrumb::show(ui, graph, state, selected.as_deref(), i18n);
    let shown = rows::show(ui, graph, state, (selected.as_deref(), focused), i18n);
    let pinned = sticky::show(ui, graph, state, shown.view, shown.step);
    let event = pinned.or(shown.event).or(crumb);

    apply(event, &source, state, selected)
}

/// Does what `event` asked for. Returns a row's menu action, for the caller to carry out.
fn apply(
    event: Option<TreeEvent>,
    source: &ObjectTree<'_>,
    state: &mut TreeState,
    selected: &mut Option<Rc<str>>,
) -> Option<(Node, RowAction)> {
    match event? {
        TreeEvent::Select(row) => {
            state.cursor = row;
            *selected = Some(source.graph.id(state.rows()[row].node).into());
            None
        }
        TreeEvent::Menu(row, picked) => {
            state.cursor = row;
            state.rows().get(row).map(|row| (row.node, picked))
        }

        TreeEvent::SetOpen(node, open) => {
            state.set_open(source, node, open);
            None
        }
        TreeEvent::SetOpenBelow(node, open) => {
            state.set_open_below(source, node, open);
            None
        }

        TreeEvent::Locate => {
            state.scroll_to = Some(state.cursor);
            None
        }
    }
}
