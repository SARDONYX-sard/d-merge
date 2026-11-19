//! The ids an object's text holds as plain numbers: states, and indices of events and variables.
//!
//! A pointer (`#0106`) names its object; these only name something with the object that gives them a
//! meaning.
//!
//! | kind              | field (class)                                                     | names                                       |
//! | ----------------- | ----------------------------------------------------------------- | ------------------------------------------- |
//! | `State`           | `stateId` (`hkbStateMachineStateInfo`)                            | the state, in its state machine             |
//! | `StartState`      | `startStateId` (`hkbStateMachine`)                                | a state of that state machine               |
//! | `ToState`         | `toStateId` (`hkbStateMachineTransitionInfo`)                     | a state of the state machine it is in       |
//! | `FromNestedState` | `fromNestedStateId`, if `FLAG_FROM_NESTED_STATE_ID_IS_VALID`      | a state of the from state's state machine   |
//! | `ToNestedState`   | `toNestedStateId`, if `FLAG_TO_NESTED_STATE_ID_IS_VALID`          | a state of the to state's state machine     |
//! | `Event`           | `id` (`hkbEvent`, `hkbEventProperty`), `eventId`, `...EventId`    | the index in `eventNames`                   |
//! | `Variable`        | `variableIndex` of a variable binding, `syncVariableIndex`, ...   | the index in `variableNames`                |
//!
//! e.g. a transition: `<hkparam name="toStateId">3</hkparam>` -> `ToState` `3`.
//!
//! A negative number is none (`-1`: no event, no variable): it is listed, to tell so, but names
//! nothing. `$eventID[x]$` is resolved by Nemesis, and not listed.

use std::ops::Range;

use crate::{attribute, schema::walk_line};

/// What an id of [`id_fields`] names. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdKind {
    State,
    StartState,
    ToState,
    FromNestedState,
    ToNestedState,
    Event,
    Variable,
}

/// One id of an object's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdField {
    pub kind: IdKind,
    /// e.g. `3`
    pub value: i64,
    /// 0-based.
    pub line: usize,
    /// The chars of the value in the line.
    pub columns: Range<usize>,
}

/// What the class's field names, if it is an id. See the module docs.
///
/// e.g. `hkbStateMachineTransitionInfo`, `toStateId` -> `ToState`
pub fn kind_of(class: &str, field: &str) -> Option<IdKind> {
    Some(match (class, field) {
        ("hkbStateMachineStateInfo", "stateId") => IdKind::State,
        ("hkbStateMachine", "startStateId") => IdKind::StartState,
        ("hkbStateMachineTransitionInfo", "toStateId") => IdKind::ToState,
        ("hkbStateMachineTransitionInfo", "fromNestedStateId") => IdKind::FromNestedState,
        ("hkbStateMachineTransitionInfo", "toNestedStateId") => IdKind::ToNestedState,
        ("hkbEvent" | "hkbEventBase" | "hkbEventProperty", "id") | (_, "eventId") => IdKind::Event,
        (_, field) if field.ends_with("EventId") => IdKind::Event,
        (
            "hkbVariableBindingSetBinding"
            | "hkbBoolVariableSequencedData"
            | "hkbIntVariableSequencedData"
            | "hkbRealVariableSequencedData",
            "variableIndex",
        )
        | ("hkbExpressionData", "assignmentVariableIndex")
        | ("hkbStateMachine", "syncVariableIndex") => IdKind::Variable,
        _ => return None,
    })
}

/// What one `<hkobject>` says about its ids, which its later fields may decide (`flags` comes after
/// `toNestedStateId`, `bindingType` after `variableIndex`).
#[derive(Default)]
struct Frame {
    ids: Vec<IdField>,
    from_nested_valid: bool,
    to_nested_valid: bool,
    /// `BINDING_TYPE_CHARACTER_PROPERTY`: its `variableIndex` names a character property.
    binds_property: bool,
}

impl Frame {
    /// Its ids that name something.
    fn into_ids(self) -> impl Iterator<Item = IdField> {
        let Self { ids, from_nested_valid, to_nested_valid, binds_property } = self;
        ids.into_iter().filter(move |id| match id.kind {
            IdKind::FromNestedState => from_nested_valid,
            IdKind::ToNestedState => to_nested_valid,
            IdKind::Variable => !binds_property,
            _ => true,
        })
    }
}

/// Every id of `object` (one object text) that is a plain number, in line order. See the module docs.
///
/// e.g. `<hkparam name="stateId">3</hkparam>` of a `hkbStateMachineStateInfo` -> `State` `3`
pub fn id_fields(object: &str) -> Vec<IdField> {
    let object = crate::open_kept_arrays(object);
    let mut found = Vec::new();
    let mut objects: Vec<Option<&str>> = Vec::new();
    let mut fields: Vec<Option<&'static str>> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();

    for (line_no, text) in object.lines().enumerate() {
        let trimmed = text.trim();
        if trimmed.starts_with("<hkobject") && !trimmed.ends_with("/>") {
            frames.push(Frame::default());
        } else if trimmed.starts_with("</hkobject>") {
            if let Some(frame) = frames.pop() {
                found.extend(frame.into_ids());
            }
        } else if let Some(frame) = frames.last_mut()
            && let Some((name, value)) = one_line_field(trimmed)
        {
            let class = objects.last().copied().flatten();
            match name {
                "flags" => {
                    frame.from_nested_valid = value.contains("FLAG_FROM_NESTED_STATE_ID_IS_VALID");
                    frame.to_nested_valid = value.contains("FLAG_TO_NESTED_STATE_ID_IS_VALID");
                }
                "bindingType" => frame.binds_property = value.contains("CHARACTER_PROPERTY"),
                _ => {
                    if let Some(kind) = class.and_then(|class| kind_of(class, name))
                        && let Ok(number) = value.trim().parse::<i64>()
                    {
                        let columns = value_columns(text, value);
                        frame.ids.push(IdField { kind, value: number, line: line_no, columns });
                    }
                }
            }
        }
        walk_line(text, &mut objects, &mut fields);
    }
    // An object cut short: what it had so far.
    found.extend(frames.into_iter().rev().flat_map(Frame::into_ids));
    found.sort_by_key(|id| id.line);
    found
}

/// `name="toStateId">3</hkparam>` -> `toStateId`, `3`.
fn one_line_field(trimmed: &str) -> Option<(&str, &str)> {
    let rest = trimmed.strip_prefix("<hkparam")?;
    let name = attribute(rest, "name")?;
    let value = rest.split_once('>')?.1.strip_suffix("</hkparam>")?;
    Some((name, value))
}

/// The chars `value` (a part of `line`) takes in it.
fn value_columns(line: &str, value: &str) -> Range<usize> {
    let byte = value.as_ptr() as usize - line.as_ptr() as usize;
    let trimmed = value.trim_start();
    let start = line[..byte].chars().count() + (value.len() - trimmed.len());
    start..start + trimmed.trim_end().chars().count()
}

/// The pointers of the field `name` of `object` (one object text), at its top level: one pointer, or
/// an array of them. Empty if it has none.
///
/// e.g. `<hkparam name="states" numelements="2">\n#0100\n#0101\n</hkparam>` -> `[#0100, #0101]`
pub fn pointers_of<'a>(object: &'a str, name: &str) -> Vec<&'a str> {
    let mut lines = object.lines();
    let Some(first) = lines.by_ref().find(|line| field_name(line) == Some(name)) else { return Vec::new() };
    let first = first.split_once('>').map_or("", |(_, value)| value);
    let mut pointers: Vec<&'a str> = Vec::new();
    for text in std::iter::once(first).chain(lines) {
        let (text, closed) = text.split_once("</hkparam>").map_or((text, false), |(text, _)| (text, true));
        pointers.extend(text.split_whitespace().filter(|word| word.starts_with('#')));
        if closed {
            break;
        }
    }
    pointers
}

/// The number of the field `name` of `object` at its top level, with its line. e.g. `stateId` -> `(3, 12)`
pub fn number_of(object: &str, name: &str) -> Option<(i64, usize)> {
    object.lines().enumerate().find_map(|(line_no, line)| {
        let (field, value) = one_line_field(line.trim())?;
        (field == name).then(|| value.trim().parse().ok().map(|number| (number, line_no))).flatten()
    })
}

/// The value of the first one-line field `name` of `object`. e.g. `name` -> `IdleState`
pub fn text_of<'a>(object: &'a str, name: &str) -> Option<&'a str> {
    object
        .lines()
        .find_map(|line| one_line_field(line.trim()).filter(|(field, _)| *field == name).map(|(_, value)| value))
}

/// `<hkparam name="states" numelements="2">` -> `states`
fn field_name(line: &str) -> Option<&str> {
    attribute(line.trim().strip_prefix("<hkparam")?, "name")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSITIONS: &str = r##"		<hkobject name="#0090" class="hkbStateMachineTransitionInfoArray" signature="0xe397b11e">
			<hkparam name="transitions" numelements="2">
				<hkobject>
					<hkparam name="eventId">12</hkparam>
					<hkparam name="toStateId">3</hkparam>
					<hkparam name="fromNestedStateId">0</hkparam>
					<hkparam name="toNestedStateId">5</hkparam>
					<hkparam name="flags">FLAG_TO_NESTED_STATE_ID_IS_VALID</hkparam>
				</hkobject>
				<hkobject>
					<hkparam name="eventId">-1</hkparam>
					<hkparam name="toStateId">$eventID[x]$</hkparam>
				</hkobject>
			</hkparam>
		</hkobject>
"##;

    #[test]
    fn a_transition_names_states_and_an_event() {
        let ids: Vec<(IdKind, i64, usize)> =
            id_fields(TRANSITIONS).iter().map(|id| (id.kind, id.value, id.line)).collect();
        let expected =
            [(IdKind::Event, 12, 3), (IdKind::ToState, 3, 4), (IdKind::ToNestedState, 5, 6), (IdKind::Event, -1, 10)];
        assert_eq!(ids, expected, "`-1`: none, listed all the same");
        assert_eq!(id_fields(TRANSITIONS)[1].columns, 31..32, "the `3`, after 5 tabs and the tag");
    }

    /// `hkbEventProperty.id` in a struct, whose class comes from its field.
    #[test]
    fn the_event_of_a_trigger_is_an_event() {
        let object = "<hkobject name=\"#0092\" class=\"hkbClipTriggerArray\" signature=\"0x59c23a0f\">\n<hkparam name=\"triggers\" numelements=\"1\">\n<hkobject>\n<hkparam name=\"event\">\n<hkobject>\n<hkparam name=\"id\">7</hkparam>\n</hkobject>\n</hkparam>\n</hkobject>\n</hkparam>\n</hkobject>\n";
        let ids: Vec<(IdKind, i64)> = id_fields(object).iter().map(|id| (id.kind, id.value)).collect();
        assert_eq!(ids, [(IdKind::Event, 7)]);
    }

    #[test]
    fn a_binding_of_a_character_property_names_no_variable() {
        let object = r##"<hkobject name="#0091" class="hkbVariableBindingSet" signature="0x338ad4ff">
<hkparam name="bindings" numelements="2">
<hkobject>
<hkparam name="variableIndex">2</hkparam>
<hkparam name="bindingType">BINDING_TYPE_VARIABLE</hkparam>
</hkobject>
<hkobject>
<hkparam name="variableIndex">0</hkparam>
<hkparam name="bindingType">BINDING_TYPE_CHARACTER_PROPERTY</hkparam>
</hkobject>
</hkparam>
</hkobject>
"##;
        let ids: Vec<(IdKind, i64)> = id_fields(object).iter().map(|id| (id.kind, id.value)).collect();
        assert_eq!(ids, [(IdKind::Variable, 2)]);
    }

    #[test]
    fn a_field_s_pointers_and_number() {
        let object = "<hkobject name=\"#0100\" class=\"hkbStateMachine\">\n<hkparam name=\"startStateId\">2</hkparam>\n<hkparam name=\"states\" numelements=\"2\">\n#0101\n#0102\n</hkparam>\n<hkparam name=\"wildcardTransitions\">#0090</hkparam>\n</hkobject>\n";
        assert_eq!(pointers_of(object, "states"), ["#0101", "#0102"]);
        assert_eq!(pointers_of(object, "wildcardTransitions"), ["#0090"]);
        assert!(pointers_of(object, "none").is_empty());
        assert_eq!(number_of(object, "startStateId"), Some((2, 1)));
        assert_eq!(text_of(object, "wildcardTransitions"), Some("#0090"));
    }
}
