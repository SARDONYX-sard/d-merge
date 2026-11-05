//! What the panels and the completion know about the active behavior: its events, variables and
//! the pointers between its objects. Kept up to date object by object.
//!
//! e.g., typing in `#0106` re-reads that one object; `hkbBehaviorGraphStringData`, which holds the
//! event and variable names, is only parsed again when it is the object that changed.

use nemesis_diff::vars;

use super::{
    tree::Graph,
    workspace::{Behavior, STRING_DATA},
};

/// See the module docs.
#[derive(Debug, Clone, Default)]
pub(super) struct Names {
    pub(super) events: Vec<String>,
    pub(super) variables: Vec<String>,
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
        names
    }

    /// Takes in the objects with these ids, which changed since the last call.
    pub(super) fn apply_changes(&mut self, behavior: &Behavior, changed: &[String]) {
        for id in changed {
            let object = behavior.object_text(id);
            self.graph.update(id, object);
            if let Some(object) = object.filter(|object| object.class == STRING_DATA) {
                self.read_names(object.text);
            }
        }
        self.graph.finish_updates();
    }

    fn read_names(&mut self, string_data: &str) {
        self.events = vars::names(string_data, "eventNames");
        self.variables = vars::names(string_data, "variableNames");
    }
}
