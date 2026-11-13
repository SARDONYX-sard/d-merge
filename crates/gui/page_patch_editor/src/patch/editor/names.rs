//! What the panels and the completion know about the active behavior: its events, variables and
//! the pointers between its objects. Kept up to date object by object.
//!
//! e.g., typing in `#0106` re-reads that one object; `hkbBehaviorGraphStringData`, which holds the
//! event and variable names, and `hkbBehaviorGraphData`, which holds the variable types, are only
//! parsed again when they are the object that changed.

use std::rc::Rc;

use nemesis_diff::vars;

use crate::patch::{
    tree::Graph,
    vars_edit::GRAPH_DATA,
    workspace::{Behavior, STRING_DATA},
};

/// The [`Names`] of the active behavior, kept up to date object by object.
///
/// ```text
/// cache.changed("#0106")       an object was edited, added or deleted: only it is read again
/// cache.get(0, &behavior)      the names of behavior 0: built, or the changes taken in
/// cache.reset()                the behaviors themselves changed (loaded, closed): built again
/// ```
/// Asked for every frame: an `Rc`, so the thousands of strings are not copied each time.
#[derive(Debug, Default)]
pub(in crate::patch) struct NamesCache {
    /// `(behavior, its names)`.
    of: Option<(usize, Rc<Names>)>,
    /// Objects changed since `of` last looked.
    changed: Vec<String>,
}

impl NamesCache {
    /// The names of behavior `index` (`behavior`), with the changes since the last call taken in.
    pub(in crate::patch) fn get(&mut self, index: usize, behavior: &Behavior) -> Rc<Names> {
        if !self.is_of(index) {
            self.reset();
        }
        let (_, names) = self.of.get_or_insert_with(|| (index, Rc::new(Names::new(behavior))));
        if !self.changed.is_empty() {
            Rc::make_mut(names).apply_changes(behavior, &self.changed);
            self.changed.clear();
        }
        Rc::clone(names)
    }

    /// Are these the names of behavior `index` (or none built yet)?
    pub(in crate::patch) fn is_of(&self, index: usize) -> bool {
        self.of.as_ref().is_none_or(|(of, _)| *of == index)
    }

    /// Object `id` was edited, added or deleted.
    pub(in crate::patch) fn changed(&mut self, id: &str) {
        if !self.changed.iter().any(|changed| changed == id) {
            self.changed.push(id.to_owned());
        }
    }

    /// Built again on the next [`Self::get`].
    pub(in crate::patch) fn reset(&mut self) {
        self.of = None;
        self.changed.clear();
    }
}

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub(in crate::patch) struct Names {
    pub(in crate::patch) events: Vec<String>,
    pub(in crate::patch) variables: Vec<String>,
    /// The type of each variable by index, e.g. `REAL`.
    pub(in crate::patch) variable_types: Vec<Box<str>>,
    /// `events` and `variables` with those of the referenced mods: what can be used, while only the
    /// others can be edited.
    pub(in crate::patch) usable_events: Vec<String>,
    pub(in crate::patch) usable_variables: Vec<String>,
    /// How many of each the template has (the rest are this mod's).
    pub(in crate::patch) template_events: usize,
    pub(in crate::patch) template_variables: usize,
    pub(in crate::patch) graph: Graph,
}

impl Names {
    pub(in crate::patch) fn new(behavior: &Behavior) -> Self {
        let mut names = Self { graph: Graph::new(behavior), ..Self::default() };
        if let Some(id) = behavior.object_of_class(STRING_DATA) {
            let template = behavior.template_object(id).map(|o| o.text.as_str()).unwrap_or_default();
            names.template_events = vars::names(template, "eventNames").len();
            names.template_variables = vars::names(template, "variableNames").len();
            names.read_names(behavior.text(id).unwrap_or_default());
        }
        names.read_usable(behavior);
        if let Some(id) = behavior.object_of_class(GRAPH_DATA) {
            names.read_types(behavior.text(id).unwrap_or_default());
        }
        names
    }

    /// Takes in the objects with these ids, which changed since the last call.
    pub(in crate::patch) fn apply_changes(&mut self, behavior: &Behavior, changed: &[String]) {
        for id in changed {
            let object = behavior.object_text(id);
            self.graph.update(id, object);
            match object {
                Some(object) if object.class == STRING_DATA => self.read_names(object.text),
                Some(object) if object.class == GRAPH_DATA => self.read_types(object.text),
                _ => {}
            }
        }
        self.read_usable(behavior);
        self.graph.finish_updates();
    }

    fn read_usable(&mut self, behavior: &Behavior) {
        self.usable_events.clone_from(&self.events);
        self.usable_events.extend_from_slice(&behavior.ref_events);
        self.usable_variables.clone_from(&self.variables);
        self.usable_variables.extend_from_slice(&behavior.ref_variables);
    }

    fn read_types(&mut self, graph_data: &str) {
        self.variable_types = vars::variable_types(graph_data).into_iter().map(Box::from).collect();
    }

    fn read_names(&mut self, string_data: &str) {
        self.events = vars::names(string_data, "eventNames").into_iter().map(str::to_owned).collect();
        self.variables = vars::names(string_data, "variableNames").into_iter().map(str::to_owned).collect();
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Added with the panel or typed into the object: the list has it.
    #[test]
    #[ignore = "needs the template of the repository's resource"]
    fn an_added_event_is_listed() {
        let path = Path::new("../../../resource/assets/templates/meshes/actors/character/behaviors/0_master.bin");
        let mut behavior = Behavior::open(path).unwrap();
        let mut names = Names::new(&behavior);
        let before = names.events.len();
        let string_data = behavior.object_of_class(STRING_DATA).unwrap().to_owned();
        let graph_data = behavior.object_of_class(GRAPH_DATA).unwrap().to_owned();

        let (string_text, graph_text) =
            vars::add_event(behavior.text(&string_data).unwrap(), behavior.text(&graph_data).unwrap(), "myEvent")
                .unwrap();
        behavior.set_text(&string_data, &string_text);
        behavior.set_text(&graph_data, &graph_text);
        names.apply_changes(&behavior, &[string_data.clone(), graph_data]);
        assert_eq!(names.events.len(), before + 1);
        assert_eq!(names.events.last().map(String::as_str), Some("myEvent"));

        let typed = string_text.replace(
            "<hkcstring>myEvent</hkcstring>",
            "<hkcstring>myEvent</hkcstring>\n\t\t\t\t<hkcstring>typed</hkcstring>",
        );
        behavior.set_text(&string_data, &typed);
        names.apply_changes(&behavior, &[string_data]);
        assert_eq!(names.events.last().map(String::as_str), Some("typed"));
    }
}
