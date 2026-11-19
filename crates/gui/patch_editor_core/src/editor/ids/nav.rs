//! Go to definition / find references of the ids named by number ([`nemesis_diff::ids`]).
//!
//! | at                                                 | definition                    | references                    |
//! | -------------------------------------------------- | ----------------------------- | ----------------------------- |
//! | `stateId` of a state                               | itself                        | what goes to that state       |
//! | `toStateId`, `startStateId`, `to/fromNestedStateId`| the `stateId` of that state   | what goes to that state       |
//! | an event id / a variable index                     | its line in `eventNames`, ... | its numbers and `$eventID[]$` |
//!
//! e.g. `<hkparam name="toStateId">1</hkparam>` of `#0091` -> `#0102` (`stateId` `1` of `#0100`).

use std::borrow::Cow;

use egui_code_editor::{Location, References};
use nemesis_diff::{
    ids::{IdField, IdKind, id_fields},
    schema, vars,
};
use stdx::text::contains_ignore_case;

use super::{IdIndex, object_id};

impl<'b> Target<'b> {
    /// The state it is, if it is one.
    pub(super) const fn state(self) -> Option<&'b str> {
        match self {
            Self::State(state) => Some(state),
            Self::Name(..) => None,
        }
    }
}

/// What a number names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target<'b> {
    /// A state (`hkbStateMachineStateInfo`). e.g. `#0102`
    State(&'b str),
    /// The index of an event (`Event`) or a variable (`Variable`). e.g. `(Event, 12)`
    Name(IdKind, usize),
}

/// Does `field` take char `column` of 0-based `line` (or end right before it)?
pub(super) fn is_at(field: &IdField, (line, column): (usize, usize)) -> bool {
    field.line == line && (field.columns.start..=field.columns.end).contains(&column)
}

/// The field a list of references shows a number in. e.g. `ToState` -> `toStateId`
const fn field_label(kind: IdKind) -> &'static str {
    match kind {
        IdKind::State => "stateId",
        IdKind::StartState => "startStateId",
        IdKind::ToState => "toStateId",
        IdKind::FromNestedState => "fromNestedStateId",
        IdKind::ToNestedState => "toNestedStateId",
        IdKind::Event => "event",
        IdKind::Variable => "variable",
    }
}

/// The array of the names a `Name` target is in. e.g. `Event` -> `eventNames`
const fn names_field(kind: IdKind) -> &'static str {
    match kind {
        IdKind::Variable => "variableNames",
        _ => "eventNames",
    }
}

impl<'b> IdIndex<'b> {
    /// The state of `machine` whose `stateId` is `value`. e.g. `#0100`, `1` -> `#0102`
    fn state_in(&self, machine: &str, value: i64) -> Option<Target<'b>> {
        self.state_with(self.states_of.get(machine)?, value, "").map(Target::State)
    }

    /// What `field` of the object `id` names. `to_state`: the `toStateId` of the transition `field`
    /// is in, if it is.
    fn target_of(&self, field: &IdField, id: &str, to_state: Option<i64>) -> Option<Target<'b>> {
        let value = field.value;
        if value < 0 {
            return None; // none: `-1`
        }
        match field.kind {
            IdKind::State => self.state_ids.get_key_value(id).map(|(&state, _)| Target::State(state)),
            IdKind::StartState => self.state_in(id, value),
            IdKind::ToState => self.state_in(self.transitions.get(id)?.0, value),
            IdKind::ToNestedState => {
                let (machine, _) = self.transitions.get(id)?;
                let to = self.state_with(self.states_of.get(machine)?, to_state?, "")?;
                self.state_in(self.nested_machine(to)?, value)
            }
            IdKind::FromNestedState => {
                let (_, from) = self.transitions.get(id)?;
                self.state_in(self.nested_machine((*from)?)?, value)
            }
            IdKind::Event | IdKind::Variable => Some(Target::Name(field.kind, usize::try_from(value).ok()?)),
        }
    }

    /// Every id of the object `(text, id)`, with what it names.
    pub(super) fn targets(&self, (text, id): (&str, &str)) -> Vec<(IdField, Option<Target<'b>>)> {
        let mut to_state = None;
        let named = |field: IdField| {
            if field.kind == IdKind::ToState {
                to_state = Some(field.value);
            }
            let target = self.target_of(&field, id, to_state);
            (field, target)
        };
        id_fields(text).into_iter().map(named).collect()
    }

    /// Where `target` is defined: the `stateId` of a state, the line of a name.
    fn location_of(&self, target: Target<'b>) -> Option<Location<'b>> {
        match target {
            Target::State(state) => {
                let (_, line) = self.state_ids.get(state)?;
                Some(Location { document: state.into(), line: *line, detail: "stateId".into() })
            }
            Target::Name(kind, at) => {
                let (id, text) = self.names?;
                let field = names_field(kind);
                let name = *vars::names(text, field).get(at)?;
                Some(Location { document: id.into(), line: vars::name_line(text, field, name)?, detail: field.into() })
            }
        }
    }

    /// The objects that may name `target`: the state machines, states and transitions for a state;
    /// every object for a name.
    fn candidates(&self, target: Target<'b>) -> Vec<(&'b str, &'b str)> {
        let Some(behavior) = self.behavior else { return Vec::new() };
        match target {
            Target::State(_) => {
                let machines = self.states_of.keys();
                let ids = machines.chain(self.state_ids.keys()).chain(self.transitions.keys());
                ids.filter_map(|&id| Some((id, behavior.text(id)?))).collect()
            }
            Target::Name(..) => behavior.texts().map(|object| (object.id, object.text)).collect(),
        }
    }
}

/// What the number at `(line, column)` of the object text `text` names, if it is an id. A name in
/// `eventNames` / `variableNames` is its own definition (so the editor offers its references).
pub fn definition<'b>(text: &str, at: (usize, usize), index: &IdIndex<'b>) -> Option<Location<'b>> {
    let id = object_id(text)?;
    index.location_of(target_at((text, id), at, index)?)
}

/// Every place naming what the number at `(line, column)` of the object text `text` names, or the
/// name there in `eventNames` / `variableNames`.
///
/// Its numbers, and for an event or a variable, its line in the names and its `$eventID[name]$` /
/// `$variableID[name]$`. `text` is taken for its object over the behavior's (it may be newer).
pub fn references<'b>(text: &str, at: (usize, usize), index: &IdIndex<'b>) -> Option<References<'b>> {
    let id = object_id(text)?;
    let target = target_at((text, id), at, index)?;

    let mut locations: Vec<Location<'b>> = Vec::new();
    for (other, other_text) in index.candidates(target) {
        let other_text = if other == id { text } else { other_text };
        let named = index.targets((other_text, other)).into_iter().filter(|(_, named)| *named == Some(target));
        locations.extend(named.map(|(field, _)| Location {
            document: other.into(),
            line: field.line,
            detail: field_label(field.kind).into(),
        }));
    }

    let symbol = match target {
        Target::State(state) => {
            let state_id = index.state_ids.get(state).map(|(value, _)| *value).unwrap_or_default();
            format!("stateId {state_id} ({state})")
        }
        Target::Name(kind, at) => {
            locations.extend(index.location_of(target)); // its line in the names
            let name = index.names.and_then(|(_, text)| vars::names(text, names_field(kind)).get(at).copied());
            if let Some(name) = name {
                let token = name_token(kind, name);
                for (other, other_text) in index.candidates(target) {
                    let uses = other_text.lines().enumerate().filter(|(_, line)| contains_ignore_case(line, &token));
                    locations.extend(uses.map(|(line, _)| Location {
                        document: other.into(),
                        line,
                        detail: token.clone().into(),
                    }));
                }
            }
            format!("{} #{at}{}", field_label(kind), name.map_or_else(String::new, |name| format!(" ({name})")))
        }
    };
    Some(References { symbol: Cow::Owned(symbol), locations })
}

/// What is at `at` of the object `(text, id)`: what its number names, or, in the names object of
/// the behavior, the event / variable of the name there.
///
/// e.g. on `<hkcstring>moveStop</hkcstring>`, element 1 of `eventNames` -> `Name(Event, 1)`
fn target_at<'b>((text, id): (&str, &str), at: (usize, usize), index: &IdIndex<'b>) -> Option<Target<'b>> {
    if let Some((_, target)) = index.targets((text, id)).into_iter().find(|(field, _)| is_at(field, at)) {
        return target;
    }
    let is_names_object = index.names.is_some_and(|(names, _)| names == id);
    let element = schema::array_element(text, at.0, at.1).filter(|_| is_names_object)?;
    let kind = match element.field {
        "eventNames" => IdKind::Event,
        "variableNames" => IdKind::Variable,
        _ => return None,
    };
    Some(Target::Name(kind, element.index))
}

/// `$eventID[name]$` / `$variableID[name]$`, lowercase (Nemesis ignores the case).
fn name_token(kind: IdKind, name: &str) -> String {
    let prefix = if kind == IdKind::Variable { vars::VARIABLE_ID_PREFIX } else { vars::EVENT_ID_PREFIX };
    format!("{prefix}{name}]$").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{super::tests::*, *};
    use crate::workspace::{Behavior, STRING_DATA};

    /// `(document, line)` of each location.
    fn places(locations: &[Location<'_>]) -> Vec<(String, usize)> {
        let mut places: Vec<(String, usize)> =
            locations.iter().map(|location| (location.document.to_string(), location.line)).collect();
        places.sort();
        places
    }

    /// The line and the column of the first `needle` in `text`.
    fn at(text: &str, needle: &str) -> (usize, usize) {
        text.lines()
            .enumerate()
            .find_map(|(line, l)| l.find(needle).map(|byte| (line, l[..byte].chars().count() + needle.len() - 1)))
            .unwrap()
    }

    fn to(state_id: i64) -> String {
        format!("<hkparam name=\"toStateId\">{state_id}</hkparam>\n")
    }

    #[test]
    fn a_transition_goes_to_its_state() {
        let behavior = machines(&to(1));
        let index = IdIndex::new(&behavior);
        let text = behavior.text("#t").unwrap();

        let location = definition(text, at(text, "\">1"), &index).unwrap();
        assert_eq!((location.document.as_ref(), location.line), ("#b", 1), "`stateId` of `#b`");
        assert!(definition(text, (0, 0), &index).is_none(), "no number there");
    }

    #[test]
    fn a_state_is_gone_to_by_its_transitions() {
        let behavior = machines(&to(0));
        let index = IdIndex::new(&behavior);
        let text = behavior.text("#a").unwrap();

        let found = references(text, at(text, "\">0"), &index).unwrap();
        assert_eq!(found.symbol, "stateId 0 (#a)");
        // Itself, the start of `#m`, its transitions and the wildcards.
        assert_eq!(places(&found.locations), [("#a".into(), 1), ("#m".into(), 1), ("#t".into(), 3), ("#w".into(), 3)]);
    }

    #[test]
    fn an_event_number_is_found_with_its_name() {
        let names = "<hkparam name=\"eventNames\" numelements=\"2\">\n<hkcstring>a</hkcstring>\n<hkcstring>b</hkcstring>\n</hkparam>\n";
        let uses = "<hkparam name=\"activateEventId\">1</hkparam>\n<hkparam name=\"deactivateEventId\">$eventID[B]$</hkparam>\n";
        let behavior: Behavior =
            behavior_of(vec![object("#s", STRING_DATA, names), object("#e", "hkbEventDrivenModifier", uses)]);
        let index = IdIndex::new(&behavior);
        let text = behavior.text("#e").unwrap();

        let location = definition(text, at(text, "\">1"), &index).unwrap();
        assert_eq!((location.document.as_ref(), location.line), ("#s", 3), "`b`");
        let found = references(text, at(text, "\">1"), &index).unwrap();
        assert_eq!(found.symbol, "event #1 (b)");
        let expected = [("#e".into(), 1), ("#e".into(), 2), ("#s".into(), 3)];
        assert_eq!(places(&found.locations), expected, "the number, the name, and its line");

        // From the name itself.
        let names_text = behavior.text("#s").unwrap();
        let found = references(names_text, at(names_text, ">b<"), &index).unwrap();
        assert_eq!(found.symbol, "event #1 (b)");
        assert_eq!(places(&found.locations), expected);
        assert!(references(names_text, (0, 0), &index).is_none(), "not on a name");
        let itself = definition(names_text, at(names_text, ">b<"), &index).unwrap();
        assert_eq!((itself.document.as_ref(), itself.line), ("#s", 3), "its own definition");
    }
}
