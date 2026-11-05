//! Adding and removing events and variables of the active behavior (drawn by
//! [`super::ui::vars_panel`]): the ones this mod added.
//!
//! Adding edits the string data, graph data (and value set) objects at once; see
//! [`nemesis_diff::vars`]. Template entries cannot be removed (other mods index them).

use std::rc::Rc;

use nemesis_diff::vars::{self, VariableType};

use super::{PatchTool, names::Names};

pub(super) const GRAPH_DATA: &str = "hkbBehaviorGraphData";
pub(super) const VALUE_SET: &str = "hkbVariableValueSet";

/// Inputs of the "add" rows.
#[derive(Debug)]
pub(super) struct NewNames {
    pub(super) event: String,
    pub(super) variable: String,
    pub(super) kind: VariableType,
    pub(super) value: String,
}

impl Default for NewNames {
    fn default() -> Self {
        Self { event: String::new(), variable: String::new(), kind: VariableType::Int32, value: String::new() }
    }
}

impl PatchTool {
    /// The active behavior's names, with the changes since the last call taken in.
    ///
    /// Asked for every frame: an `Rc`, so the thousands of strings are not copied each time.
    pub(super) fn names(&mut self) -> Rc<Names> {
        let active = self.active;
        if self.names.as_ref().is_some_and(|(cached, _)| *cached != active) {
            self.reset_names();
        }
        let behavior = &self.workspace.behaviors[active];
        let (_, names) = self.names.get_or_insert_with(|| (active, Rc::new(Names::new(behavior))));
        if !self.changed_objects.is_empty() {
            Rc::make_mut(names).apply_changes(behavior, &self.changed_objects);
            self.changed_objects.clear();
        }
        Rc::clone(names)
    }

    /// Returns the ids of the objects it edited.
    pub(super) fn apply(
        &mut self,
        action: Action,
        ids: &Ids,
        events: &[String],
        variables: &[String],
    ) -> Result<Vec<String>, String> {
        let behavior = &mut self.workspace.behaviors[self.active];
        let text = |id: &str| behavior.text(id).unwrap_or_default().to_string();
        let new = &mut self.new_names;
        let mut edits: Vec<(String, String)> = Vec::new();
        match action {
            Action::AddEvent => {
                let name = valid_name(&new.event, events)?;
                let (s, g) = vars::add_event(&text(&ids.string_data), &text(&ids.graph_data), name)
                    .ok_or("eventNames / eventInfos not found")?;
                edits = vec![(ids.string_data.clone(), s), (ids.graph_data.clone(), g)];
                new.event.clear();
            }
            Action::AddVariable => {
                let name = valid_name(&new.variable, variables)?;
                let value = new.kind.word_value(&new.value).ok_or_else(|| format!("Invalid value: `{}`", new.value))?;
                let value_set = ids.value_set.as_deref().ok_or("No hkbVariableValueSet")?;
                let (s, g, v) = vars::add_variable(
                    &text(&ids.string_data),
                    &text(&ids.graph_data),
                    &text(value_set),
                    name,
                    new.kind,
                    value,
                )
                .ok_or("variableNames / variableInfos / wordVariableValues not found")?;
                edits = vec![(ids.string_data.clone(), s), (ids.graph_data.clone(), g), (value_set.to_string(), v)];
                new.variable.clear();
                new.value.clear();
            }
            Action::RemoveEvent(i) => {
                for (id, param) in [(&ids.string_data, "eventNames"), (&ids.graph_data, "eventInfos")] {
                    edits.push((id.clone(), vars::remove(&text(id), param, i).ok_or(param)?));
                }
            }
            Action::RemoveVariable(i) => {
                let mut targets = vec![(&ids.string_data, "variableNames"), (&ids.graph_data, "variableInfos")];
                if let Some(value_set) = &ids.value_set {
                    targets.push((value_set, "wordVariableValues"));
                }
                for (id, param) in targets {
                    edits.push((id.clone(), vars::remove(&text(id), param, i).ok_or(param)?));
                }
            }
        }
        let changed = edits.iter().map(|(id, _)| id.clone()).collect();
        for (id, edited) in edits {
            behavior.set_text(&id, edited);
        }
        self.vars_error = None;
        Ok(changed)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Action {
    AddEvent,
    AddVariable,
    RemoveEvent(usize),
    RemoveVariable(usize),
}

pub(super) struct Ids {
    pub(super) string_data: String,
    pub(super) graph_data: String,
    pub(super) value_set: Option<String>,
}

/// A name to add, trimmed: no whitespace, not there yet. Nemesis ignores ASCII case in names.
///
/// e.g., ` myVar ` -> `myVar`; `MOVESTART` when `moveStart` exists -> an error.
fn valid_name<'a>(name: &'a str, existing: &[String]) -> Result<&'a str, String> {
    let name = name.trim();
    if name.is_empty() || name.contains(char::is_whitespace) {
        return Err(format!("Invalid name: `{name}`"));
    }
    if existing.iter().any(|n| n.eq_ignore_ascii_case(name)) {
        return Err(format!("Already exists: `{name}`"));
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_new_names() {
        let existing = ["moveStart".to_owned()];
        assert_eq!(valid_name(" myVar ", &existing), Ok("myVar"));
        assert!(valid_name("", &existing).is_err());
        assert!(valid_name("my var", &existing).is_err());
        assert!(valid_name("MOVESTART", &existing).is_err());
    }
}
