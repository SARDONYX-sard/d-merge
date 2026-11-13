//! What is open is part of the work: the behavior shown, the object selected, the tabs. It is
//! written with the work ([`Session`]) and opened again with it.
//!
//! ```text
//! save:  (behaviors, tabs, active, selected, place) -> Session   behaviors by name: their order may change
//! load:  Session -> tabs restored, Restored { active, selected, mod_dir, output }
//! ```
//! What no longer exists (a behavior removed, an object deleted) is left out.

use std::rc::Rc;

use crate::patch::{
    editor::tabs::EditorTabs,
    tool::place::WorkPlace,
    workspace::{Behavior, Session, SessionTab},
};

/// What is open now, to write with the work.
pub(in crate::patch) fn of(
    behaviors: &[Behavior],
    tabs: &EditorTabs,
    (active, selected): (usize, Option<&str>),
    place: &WorkPlace,
) -> Session {
    let name_of = |at: usize| behaviors.get(at).map_or_else(String::new, |behavior| behavior.name.clone());
    Session {
        mod_dir: place.mod_dir.clone(),
        output: place.output.clone(),
        active: name_of(active),
        selected: selected.map(str::to_owned),
        tabs: tabs
            .all()
            .map(|tab| SessionTab { behavior: name_of(tab.group), id: tab.id.clone(), pinned: tab.pinned })
            .collect(),
    }
}

/// What a session opens again, besides its tabs.
#[derive(Debug)]
pub(in crate::patch) struct Restored {
    pub(in crate::patch) active: usize,
    pub(in crate::patch) selected: Option<Rc<str>>,
    /// Empty: keep the one there is.
    pub(in crate::patch) mod_dir: String,
    pub(in crate::patch) output: String,
}

/// The tabs of `session` into `tabs`, and what else it had open, of what still exists in
/// `behaviors`.
pub(in crate::patch) fn restore(session: Session, behaviors: &[Behavior], tabs: &mut EditorTabs) -> Restored {
    let at_of = |name: &str| behaviors.iter().position(|behavior| behavior.name == name);
    let exists = |at: usize, id: &str| behaviors.get(at).is_some_and(|behavior| behavior.text(id).is_some());

    for tab in &session.tabs {
        if let Some(at) = at_of(&tab.behavior)
            && exists(at, &tab.id)
        {
            tabs.restore(at, &tab.id, tab.pinned);
        }
    }

    let active = at_of(&session.active).unwrap_or(0);
    let selected = session.selected.filter(|id| exists(active, id)).map(Rc::from);
    Restored { active, selected, mod_dir: session.mod_dir, output: session.output }
}
