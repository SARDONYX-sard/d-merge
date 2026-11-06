//! Adding and removing events and variables of the active behavior (drawn by
//! [`super::ui::vars_panel`]): the ones this mod added.
//!
//! Adding edits the string data, graph data (and value set) objects at once; see
//! [`nemesis_diff::vars`]. Template entries cannot be removed (other mods index them).

use std::rc::Rc;

use nemesis_diff::vars::{self, VariableType};

use super::{PatchTool, editor::names::Names};

pub(super) const GRAPH_DATA: &str = "hkbBehaviorGraphData";
pub(super) const VALUE_SET: &str = "hkbVariableValueSet";

/// Inputs of the "add" rows.
#[derive(Debug)]
pub(super) struct NewNames {
    pub(super) event: String,
    pub(super) variable: String,
    pub(super) kind: VariableType,
    pub(super) value: String,
    /// The name being renamed in the list.
    pub(super) renaming: Option<Renaming>,
}

impl Default for NewNames {
    fn default() -> Self {
        Self {
            event: String::new(),
            variable: String::new(),
            kind: VariableType::Int32,
            value: String::new(),
            renaming: None,
        }
    }
}

/// A row of the list being renamed.
#[derive(Debug)]
pub(super) struct Renaming {
    pub(super) is_variable: bool,
    pub(super) index: usize,
    /// The field's text.
    pub(super) text: String,
    /// Give the field the keyboard on the next draw.
    pub(super) focus: bool,
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
            Action::Rename { is_variable, index } => {
                let text = new.renaming.take().map(|renaming| renaming.text).unwrap_or_default();
                let (param, prefix, existing) = if is_variable {
                    ("variableNames", vars::VARIABLE_ID_PREFIX, variables)
                } else {
                    ("eventNames", vars::EVENT_ID_PREFIX, events)
                };
                let old = existing.get(index).ok_or(param)?;
                let new_name = valid_rename(&text, existing, index)?;
                if new_name == old {
                    return Ok(Vec::new());
                }
                let renamed = vars::rename(&behavior_text(behavior, &ids.string_data), param, index, new_name);
                edits.push((ids.string_data.clone(), renamed.ok_or(param)?));
                // Patches name it: those texts change too, or they would point to nothing.
                edits.extend(behavior.texts().filter_map(|object| {
                    let edited = vars::rename_uses(object.text, prefix, old, new_name)?;
                    Some((object.id.to_owned(), edited))
                }));
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
            behavior.set_text(&id, &edited);
        }
        self.vars_error = None;
        Ok(changed)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Action {
    AddEvent,
    /// Takes the new name from [`NewNames::renaming`].
    Rename {
        is_variable: bool,
        index: usize,
    },
    AddVariable,
    RemoveEvent(usize),
    RemoveVariable(usize),
}

pub(super) struct Ids {
    pub(super) string_data: String,
    pub(super) graph_data: String,
    pub(super) value_set: Option<String>,
}

/// The text of object `id`.
fn behavior_text(behavior: &super::Behavior, id: &str) -> String {
    behavior.text(id).unwrap_or_default().to_owned()
}

/// A new name for entry `index`: as [`valid_name`], but its own old name does not count, so that
/// only the case can be changed.
fn valid_rename<'a>(name: &'a str, existing: &[String], index: usize) -> Result<&'a str, String> {
    let name = valid_name(name, &[])?;
    let taken = existing.iter().enumerate().any(|(i, other)| i != index && other.eq_ignore_ascii_case(name));
    if taken { Err(format!("Already exists: `{name}`")) } else { Ok(name) }
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

    #[test]
    fn a_rename_may_change_the_case_of_its_own_name() {
        let existing = ["moveStart".to_owned(), "moveStop".to_owned()];
        assert_eq!(valid_rename("MOVESTART", &existing, 0), Ok("MOVESTART"));
        assert!(valid_rename("moveStop", &existing, 0).is_err());
        assert!(valid_rename("my var", &existing, 0).is_err());
    }
}
