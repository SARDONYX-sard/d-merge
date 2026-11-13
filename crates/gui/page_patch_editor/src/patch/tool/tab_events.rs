//! What a tab click or a tab key does to the tabs: the objects' ([`EditorTabs`]) and the diffs' /
//! blames' ([`Views`]) of the active behavior. They see only the tabs; what the tool must do after
//! (select another object, drop the editor's state) is returned ([`TabOutcome`]).
//!
//! ```text
//! [ Root ✖ ][ GetUpModList ● ][ #0106 (#2) ↔ (working) 🔒 ✖ ]
//!   object tabs                  view tabs, after them
//! ```

use std::rc::Rc;

use egui_widgets::TabAction;

use crate::patch::{editor::tabs::EditorTabs, ui::revision_view::Views};

/// Which object is selected after a tab event.
#[derive(Debug, Default, PartialEq, Eq)]
pub(in crate::patch) enum Select {
    /// The one selected stays.
    #[default]
    Keep,
    Object(Rc<str>),
    /// None is: the tabs are gone.
    Nothing,
}

/// What the tool must do after a tab event.
#[derive(Debug, Default, PartialEq, Eq)]
pub(in crate::patch) struct TabOutcome {
    pub(in crate::patch) select: Select,
    /// The open object's tab went: its editor state with it.
    pub(in crate::patch) close_editor: bool,
    /// The editor keeps the keyboard (an object's tab was gone to by a key).
    pub(in crate::patch) keep_focus: bool,
}

impl TabOutcome {
    fn select(id: impl Into<Rc<str>>) -> Self {
        Self { select: Select::Object(id.into()), ..Self::default() }
    }
}

/// What an object's tab asked for. `current`: the object shown now.
pub(in crate::patch) fn object_action(
    (tabs, views): (&mut EditorTabs, &mut Views),
    active: usize,
    (action, id): (TabAction, String),
    current: &str,
) -> TabOutcome {
    match action {
        TabAction::Show => {
            views.list.active = None;
            TabOutcome::select(id)
        }
        TabAction::KeepOpen => {
            tabs.pin(active, &id);
            TabOutcome::default()
        }
        TabAction::Close => {
            let neighbor = tabs.close(active, &id);
            if id != current {
                return TabOutcome::default();
            }
            let select = neighbor.map_or(Select::Nothing, |id| Select::Object(id.into()));
            TabOutcome { select, close_editor: true, keep_focus: false }
        }
        TabAction::CloseOthers => {
            tabs.close_others(active, &id);
            TabOutcome::select(id)
        }
        TabAction::CloseToTheRight => {
            tabs.close_to_the_right(active, &id);
            let still_open = tabs.of(active).any(|tab| tab.id == current);
            if still_open { TabOutcome::default() } else { TabOutcome::select(id) }
        }
        TabAction::CloseAll => close_all((tabs, views), active),
    }
}

/// What a diff's / blame's tab asked for: as an object's, by its index in the views.
pub(in crate::patch) fn view_action(
    (tabs, views): (&mut EditorTabs, &mut Views),
    active: usize,
    (action, at): (TabAction, usize),
) -> TabOutcome {
    match action {
        TabAction::Show => {
            let Some(view) = views.list.get(at) else { return TabOutcome::default() };
            let outcome = TabOutcome::select(view.id.as_str());
            views.show(at);
            outcome
        }
        TabAction::KeepOpen => {
            views.list.pin(at);
            TabOutcome::default()
        }
        TabAction::Close => {
            views.list.close(at);
            TabOutcome::default()
        }
        // Every other tab of the behavior, objects' and views'.
        TabAction::CloseOthers => {
            tabs.close_all(active);
            views.close_others(at);
            let shown = views.list.active.and_then(|shown| views.list.get(shown));
            shown.map_or_else(TabOutcome::default, |view| TabOutcome::select(view.id.as_str()))
        }
        TabAction::CloseToTheRight => {
            views.list.close_to_the_right(at);
            TabOutcome::default()
        }
        TabAction::CloseAll => close_all((tabs, views), active),
    }
}

/// Every tab of the behavior closes, objects' and views'.
fn close_all((tabs, views): (&mut EditorTabs, &mut Views), active: usize) -> TabOutcome {
    tabs.close_all(active);
    views.list.close_all(active);
    TabOutcome { select: Select::Nothing, close_editor: true, keep_focus: false }
}

/// The next / previous tab, the views' after the objects', around the ends. `current`: the object
/// shown, if no view is.
pub(in crate::patch) fn step(
    (tabs, views): (&EditorTabs, &mut Views),
    active: usize,
    current: &str,
    forward: bool,
) -> TabOutcome {
    /// A tab of either kind, in the order of the bar.
    #[derive(Clone)]
    enum Tab {
        Object(String),
        View(usize),
    }

    let order: Vec<Tab> = {
        let objects = tabs.of(active).map(|tab| Tab::Object(tab.id.clone()));
        objects.chain(views.tabs_of(active).into_iter().map(|tab| Tab::View(tab.at))).collect()
    };
    let here = order.iter().position(|tab| match (tab, views.list.active) {
        (Tab::View(at), Some(shown)) => *at == shown,
        (Tab::Object(id), None) => id == current,
        _ => false,
    });
    let Some(here) = here else { return TabOutcome::default() };

    let len = order.len();
    let next = if forward { (here + 1) % len } else { (here + len - 1) % len };
    match order[next].clone() {
        Tab::Object(id) => {
            views.list.active = None;
            TabOutcome { keep_focus: true, ..TabOutcome::select(id) }
        }
        Tab::View(at) => {
            // Its own editor takes the keyboard: see `Views::show`.
            let outcome =
                views.list.get(at).map_or_else(TabOutcome::default, |view| TabOutcome::select(view.id.as_str()));
            views.show(at);
            outcome
        }
    }
}
