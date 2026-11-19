//! What the editor tells and offers of the ids named by number ([`nemesis_diff::ids`]): a hover of
//! what a number names, and the states a state field can take.
//!
//! ```text
//! <hkparam name="toStateId">1</hkparam>     hover: stateId 1: `#0102` IdleState, of `#0100` Locomotion
//! <hkparam name="toStateId">|               completion: 0 `#0101` Walk, 1 `#0102` IdleState, ...
//! <hkparam name="eventId">-1</hkparam>      hover: none
//! ```

use nemesis_diff::{
    ids::{IdKind, id_fields, kind_of, text_of},
    schema,
};

use super::{IdIndex, nav::is_at, object_id};
use crate::workspace::Behavior;

/// What the number at a place of an object's text is. See [`info_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdInfo<'b> {
    /// `-1`: none.
    Nothing(IdKind),
    /// A state, with its `name` and its state machine. e.g. `#0102`, `IdleState`, `#0100`
    State { state: &'b str, name: &'b str, machine: &'b str },
    /// The index of an event or a variable. e.g. `(Event, 12)`
    Name(IdKind, usize),
    /// It names nothing that is there. e.g. `toStateId` `7` of a state machine without it
    Missing(IdKind),
}

/// The number at `(line, column)` of the object text `text` and what it is, if it is an id.
///
/// e.g. on the `1` of `<hkparam name="toStateId">1</hkparam>` -> `(1, State { #0102, .. })`
pub fn info_at<'b>(text: &str, at: (usize, usize), behavior: &'b Behavior) -> Option<(i64, IdInfo<'b>)> {
    let field = id_fields(text).into_iter().find(|field| is_at(field, at))?;
    if field.value < 0 {
        return Some((field.value, IdInfo::Nothing(field.kind)));
    }
    if let IdKind::Event | IdKind::Variable = field.kind {
        return Some((field.value, IdInfo::Name(field.kind, usize::try_from(field.value).ok()?)));
    }

    // Only a state needs the state machines read.
    let index = IdIndex::new(behavior);
    let id = object_id(text)?;
    let target = index.targets((text, id)).into_iter().find(|(other, _)| other.line == field.line);
    let state = target.and_then(|(_, target)| target?.state());
    let info = state.map_or(IdInfo::Missing(field.kind), |state| index.state_info(state));
    Some((field.value, info))
}

/// One state a state field can take. Owned: the editor keeps the list while the cursor stays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateChoice {
    /// e.g. `1`
    pub state_id: i64,
    /// e.g. `` `#0102` IdleState ``
    pub label: String,
}

/// The states the state field on 0-based `line` of the object text `text` can name, by `stateId`.
/// Empty if the line has no state field, or its state machine is not known.
///
/// | field                 | the states of                                  |
/// | --------------------- | ---------------------------------------------- |
/// | `startStateId`        | this state machine                             |
/// | `toStateId`           | the state machine of these transitions         |
/// | `toNestedStateId`     | the state machine the `toStateId` state goes in |
/// | `fromNestedStateId`   | the state machine the from state goes in        |
pub fn state_choices(text: &str, line: usize, index: &IdIndex<'_>) -> Vec<StateChoice> {
    let Some(machine) = machine_of_field(text, line, index) else { return Vec::new() };
    let mut choices: Vec<StateChoice> = index
        .states_of
        .get(machine)
        .into_iter()
        .flatten()
        .filter_map(|&state| {
            let (state_id, _) = *index.state_ids.get(state)?;
            let IdInfo::State { name, .. } = index.state_info(state) else { return None };
            Some(StateChoice { state_id, label: format!("`{state}` {name}") })
        })
        .collect();
    choices.sort_by_key(|choice| choice.state_id);
    choices
}

/// The state machine whose states the state field on `line` names. See [`state_choices`].
fn machine_of_field<'b>(text: &str, line: usize, index: &IdIndex<'b>) -> Option<&'b str> {
    let source = text.lines().nth(line)?;
    let name = source.trim().strip_prefix("<hkparam name=\"")?.split_once('"')?.0;
    let (class, _) = schema::field_context(text, line)?;
    let id = object_id(text)?;

    match kind_of(class, name)? {
        IdKind::StartState => index.states_of.get_key_value(id).map(|(&machine, _)| machine),
        IdKind::ToState => Some(index.transitions.get(id)?.0),
        IdKind::ToNestedState => {
            // The `toStateId` of the same transition, above this line.
            let fields = id_fields(text);
            let to = fields.iter().rev().find(|field| field.line < line && field.kind == IdKind::ToState)?;
            let (machine, _) = index.transitions.get(id)?;
            let state = index.state_with(index.states_of.get(machine)?, to.value, "")?;
            index.nested_machine(state)
        }
        IdKind::FromNestedState => index.nested_machine(index.transitions.get(id)?.1?),
        IdKind::State | IdKind::Event | IdKind::Variable => None,
    }
}

impl<'b> IdIndex<'b> {
    /// `state` with its name and state machine.
    fn state_info(&self, state: &'b str) -> IdInfo<'b> {
        let text = self.behavior.and_then(|behavior| behavior.text(state)).unwrap_or_default();
        let name = text_of(text, "name").unwrap_or_default();
        let machine = self.machine_of.get(state).copied().unwrap_or_default();
        IdInfo::State { state, name, machine }
    }
}

#[cfg(test)]
mod tests {
    use super::{super::tests::*, *};

    fn to(fields: &str) -> String {
        format!("<hkparam name=\"toStateId\">1</hkparam>\n{fields}")
    }

    #[test]
    fn a_state_number_tells_its_state() {
        let behavior = machines(&to("<hkparam name=\"eventId\">-1</hkparam>\n"));
        let text = behavior.text("#t").unwrap();

        let (value, info) = info_at(text, (3, 26), &behavior).unwrap();
        assert_eq!((value, info), (1, IdInfo::State { state: "#b", name: "", machine: "#m" }));
        assert_eq!(info_at(text, (4, 24), &behavior), Some((-1, IdInfo::Nothing(IdKind::Event))));
    }

    #[test]
    fn a_state_field_offers_the_states_it_can_name() {
        let nested = "<hkparam name=\"toNestedStateId\">0</hkparam>\n";
        let behavior = machines(&to(nested));
        let index = IdIndex::new(&behavior);
        let text = behavior.text("#t").unwrap();

        let ids =
            |line| state_choices(text, line, &index).into_iter().map(|choice| choice.state_id).collect::<Vec<_>>();
        assert_eq!(ids(3), [0, 1], "`toStateId`: the states of `#m`");
        assert_eq!(ids(4), [5], "`toNestedStateId`: of `#n`, in `#b`");
        assert_eq!(state_choices(text, 3, &index)[1].label, "`#b` ");
        assert!(ids(0).is_empty(), "no state field");
    }
}
