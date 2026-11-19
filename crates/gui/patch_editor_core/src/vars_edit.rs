//! Adding, editing and removing events and variables of the active behavior (drawn by the page's
//! side bar).
//!
//! Adding edits the string data, graph data (and value set) objects at once; see
//! [`nemesis_diff::vars`]. Only this mod's entries are changed here: other mods know the
//! template's by their names, indices and types, and count on their initial values. One that must
//! be changed all the same is edited in its object's text, where its diff is seen.
//!
//! | this mod's entry | name | type | initial value | remove |
//! | ---------------- | ---- | ---- | ------------- | ------ |
//! | variable         | yes  | yes  | yes           | yes    |
//! | event            | yes  |      |               | yes    |

use nemesis_diff::vars::{self, VariableType};

use super::workspace::Behavior;

pub const GRAPH_DATA: &str = "hkbBehaviorGraphData";
pub const VALUE_SET: &str = "hkbVariableValueSet";

/// Inputs of the "add" rows.
#[derive(Debug)]
pub struct NewNames {
    pub event: String,
    pub variable: String,
    pub kind: VariableType,
    pub value: String,
    /// The row being edited in the list.
    pub editing: Option<Editing>,
}

impl Default for NewNames {
    fn default() -> Self {
        Self {
            event: String::new(),
            variable: String::new(),
            kind: VariableType::Int32,
            value: String::new(),
            editing: None,
        }
    }
}

/// A row of the list being edited: `name: TYPE = value` of a variable, the name of an event.
#[derive(Debug)]
pub struct Editing {
    pub is_variable: bool,
    pub index: usize,
    /// The fields' texts. e.g. `myVar`, `0.2`
    pub name: String,
    pub value: String,
    /// A variable's type. `None`: an event, or a type without an initial value here (`POINTER`).
    pub kind: Option<VariableType>,
    /// Give the first field the keyboard on the next draw.
    pub focus: bool,
}

/// Does `action` to `behavior`, with what `new` holds (the "add" rows, the rename field). Returns
/// the ids of the objects it edited.
///
/// # Errors
/// The name is not valid, or is taken already.
pub fn apply(
    behavior: &mut Behavior,
    new: &mut NewNames,
    action: Action,
    ids: &Ids,
    (events, variables): (&[String], &[String]),
) -> Result<Vec<String>, String> {
    let text = |id: &str| behavior.text(id).unwrap_or_default().to_string();
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
        Action::Edit => {
            let Some(editing) = new.editing.take() else { return Ok(Vec::new()) };
            match edit_edits(behavior, &editing, (ids, (events, variables))) {
                Ok(edited) => edits = edited,
                Err(message) => {
                    new.editing = Some(editing); // to be fixed
                    return Err(message);
                }
            }
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
    Ok(changed)
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    AddEvent,
    /// Takes the row (this mod's) and what is typed from [`NewNames::editing`].
    Edit,
    AddVariable,
    RemoveEvent(usize),
    RemoveVariable(usize),
}

#[derive(Debug)]
pub struct Ids {
    pub string_data: String,
    pub graph_data: String,
    pub value_set: Option<String>,
}

/// The edits of the objects that `editing` (of this mod's entry `index`) asks for: its name, and a
/// variable's type and initial value.
fn edit_edits(
    behavior: &Behavior,
    editing: &Editing,
    (ids, (events, variables)): (&Ids, (&[String], &[String])),
) -> Result<Vec<(String, String)>, String> {
    let index = editing.index;
    let existing = if editing.is_variable { variables } else { events };
    let mut edits = rename_edits(behavior, ids, (existing, index), &editing.name, editing.is_variable)?;
    let Some(kind) = editing.kind.filter(|_| editing.is_variable) else { return Ok(edits) };

    let graph_data = behavior_text(behavior, &ids.graph_data);
    let typed = vars::set_element_field(&graph_data, "variableInfos", index, "type", kind.havok_name());
    let typed = typed.ok_or("variableInfos")?;
    if typed != graph_data {
        edits.push((ids.graph_data.clone(), typed));
    }
    let word = kind.word_value(&editing.value).ok_or_else(|| format!("Invalid value: `{}`", editing.value))?;
    let value_set = ids.value_set.as_deref().ok_or("No hkbVariableValueSet")?;
    let text = behavior_text(behavior, value_set);
    let set = vars::set_element_field(&text, "wordVariableValues", index, "value", &word.to_string());
    let set = set.ok_or("wordVariableValues")?;
    if set != text {
        edits.push((value_set.to_owned(), set));
    }
    Ok(edits)
}

/// The edits renaming entry `index` of `existing` to `name`: the names object, and every use of the
/// old name (patches name it: they would point to nothing). None if the name is the same.
fn rename_edits(
    behavior: &Behavior,
    ids: &Ids,
    (existing, index): (&[String], usize),
    name: &str,
    is_variable: bool,
) -> Result<Vec<(String, String)>, String> {
    let (param, prefix) =
        if is_variable { ("variableNames", vars::VARIABLE_ID_PREFIX) } else { ("eventNames", vars::EVENT_ID_PREFIX) };
    let old = existing.get(index).ok_or(param)?;
    let new_name = valid_rename(name, existing, index)?;
    if new_name == old {
        return Ok(Vec::new());
    }
    let renamed = vars::rename(&behavior_text(behavior, &ids.string_data), param, index, new_name);
    let mut edits = vec![(ids.string_data.clone(), renamed.ok_or(param)?)];
    edits.extend(behavior.texts().filter_map(|object| {
        let edited = vars::rename_uses(object.text, prefix, old, new_name)?;
        Some((object.id.to_owned(), edited))
    }));
    Ok(edits)
}

/// The text of object `id`.
fn behavior_text(behavior: &Behavior, id: &str) -> String {
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

    /// A behavior with the variable `a` (`INT32` `5`), and its objects' ids.
    fn with_variable() -> (Behavior, Ids) {
        use std::path::Path;

        let mut behavior =
            Behavior::open(&Path::new("meshes").join(format!("{}.bin", crate::workspace::ADSF))).unwrap();
        let object = |id: &str, class: &str, field: &str| {
            format!(
                "<hkobject name=\"{id}\" class=\"{class}\">\n<hkparam name=\"{field}\" numelements=\"0\"></hkparam>\n</hkobject>\n"
            )
        };
        let string_data = object("#s", crate::workspace::STRING_DATA, "variableNames");
        let (s, g, v) = vars::add_variable(
            &string_data,
            &object("#g", GRAPH_DATA, "variableInfos"),
            &object("#v", VALUE_SET, "wordVariableValues"),
            "a",
            VariableType::Int32,
            5,
        )
        .unwrap();
        behavior.added.extend([("#s".to_owned(), s), ("#g".to_owned(), g), ("#v".to_owned(), v)]);
        let ids = Ids { string_data: "#s".to_owned(), graph_data: "#g".to_owned(), value_set: Some("#v".to_owned()) };
        (behavior, ids)
    }

    fn edit(
        behavior: &mut Behavior,
        ids: &Ids,
        (name, kind, value): (&str, VariableType, &str),
    ) -> Result<Vec<String>, String> {
        let mut new = NewNames {
            editing: Some(Editing {
                is_variable: true,
                index: 0,
                name: name.to_owned(),
                value: value.to_owned(),
                kind: Some(kind),
                focus: false,
            }),
            ..NewNames::default()
        };
        apply(behavior, &mut new, Action::Edit, ids, (&[], &["a".to_owned()]))
    }

    #[test]
    fn this_mod_s_variable_changes_its_name_type_and_value() {
        let (mut behavior, ids) = with_variable();
        let mut changed = edit(&mut behavior, &ids, ("b", VariableType::Real, "0.2")).unwrap();
        changed.sort();
        assert_eq!(changed, ["#g", "#s", "#v"]);
        assert_eq!(vars::names(behavior.text("#s").unwrap(), "variableNames"), ["b"]);
        assert_eq!(vars::variable_types(behavior.text("#g").unwrap()), ["REAL"]);
        assert_eq!(vars::word_values(behavior.text("#v").unwrap()), [1_045_220_557]);
    }

    #[test]
    fn a_value_not_of_its_type_keeps_the_row() {
        let (mut behavior, ids) = with_variable();
        assert!(edit(&mut behavior, &ids, ("a", VariableType::Int32, "x")).is_err());
        assert_eq!(vars::word_values(behavior.text("#v").unwrap()), [5], "unchanged");
    }

    #[test]
    fn a_rename_may_change_the_case_of_its_own_name() {
        let existing = ["moveStart".to_owned(), "moveStop".to_owned()];
        assert_eq!(valid_rename("MOVESTART", &existing, 0), Ok("MOVESTART"));
        assert!(valid_rename("moveStop", &existing, 0).is_err());
        assert!(valid_rename("my var", &existing, 0).is_err());
    }
}
