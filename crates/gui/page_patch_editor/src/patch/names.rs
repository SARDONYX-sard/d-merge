//! What the panels and the completion know about the active behavior: its events, variables and
//! the pointers between its objects. Kept up to date object by object.
//!
//! e.g., typing in `#0106` re-reads that one object; `hkbBehaviorGraphStringData`, which holds the
//! event and variable names, and `hkbBehaviorGraphData`, which holds the variable types, are only
//! parsed again when they are the object that changed.

use nemesis_diff::vars;

use super::{
    tree::Graph,
    vars_edit::GRAPH_DATA,
    workspace::{Behavior, STRING_DATA},
};

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub(super) struct Names {
    pub(super) events: Vec<String>,
    pub(super) variables: Vec<String>,
    /// The type of each variable by index, e.g. `REAL`.
    pub(super) variable_types: Vec<Box<str>>,
    /// `events` and `variables` with those of the referenced mods: what can be used, while only the
    /// others can be edited.
    pub(super) usable_events: Vec<String>,
    pub(super) usable_variables: Vec<String>,
    /// How many of each the template has (the rest are this mod's).
    pub(super) template_events: usize,
    pub(super) template_variables: usize,
    pub(super) graph: Graph,
}

impl Names {
    pub(super) fn new(behavior: &Behavior) -> Self {
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
    pub(super) fn apply_changes(&mut self, behavior: &Behavior, changed: &[String]) {
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
        self.events = vars::names(string_data, "eventNames");
        self.variables = vars::names(string_data, "variableNames");
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
        behavior.set_text(&string_data, string_text.clone());
        behavior.set_text(&graph_data, graph_text);
        names.apply_changes(&behavior, &[string_data.clone(), graph_data]);
        assert_eq!(names.events.len(), before + 1);
        assert_eq!(names.events.last().map(String::as_str), Some("myEvent"));

        let typed = string_text.replace(
            "<hkcstring>myEvent</hkcstring>",
            "<hkcstring>myEvent</hkcstring>\n\t\t\t\t<hkcstring>typed</hkcstring>",
        );
        behavior.set_text(&string_data, typed);
        names.apply_changes(&behavior, &[string_data]);
        assert_eq!(names.events.last().map(String::as_str), Some("typed"));
    }
}
