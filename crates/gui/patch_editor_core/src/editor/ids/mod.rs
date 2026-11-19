//! The ids an object names by number ([`nemesis_diff::ids`]), checked against the behavior: what a
//! number names is only known with the objects around it.
//!
//! ```text
//! #0100 hkbStateMachine            states: #0101 #0102     wildcardTransitions: #0090
//! #0101 hkbStateMachineStateInfo   stateId: 0              transitions: #0091
//! #0102 hkbStateMachineStateInfo   stateId: 1
//! #0091 (transitions of #0101)     toStateId: 1  -> #0102, a state of #0100
//! #0090 (wildcards of #0100)       toStateId: 7  -> none: an error
//! ```
//!
//! [`IdIndex`] is read once from the behavior; [`id_diagnostics`] checks one object's text with it,
//! and [`nav`] goes from a number to what it names and back. What can't be placed (an array of
//! transitions nothing points to) is not checked.

pub mod assist;
pub mod nav;

use egui_code_editor::{Diagnostic, Severity};
use fast_hash::HashMap;
use nemesis_diff::ids::{IdField, id_fields, number_of, pointers_of};
pub use nemesis_diff::ids::{IdKind, kind_of};

use crate::workspace::{Behavior, STRING_DATA};

const STATE_MACHINE: &str = "hkbStateMachine";
const STATE_INFO: &str = "hkbStateMachineStateInfo";

/// The objects a state's generator may be wrapped in before its state machine, and the field that
/// goes on. e.g. a state's `hkbModifierGenerator` -> its `generator`
const GENERATOR_WRAPPERS: [(&str, &str); 2] =
    [("hkbModifierGenerator", "generator"), ("BSiStateTaggingGenerator", "pDefaultGenerator")];

/// What the numbers of a behavior name. Borrows from it.
#[derive(Debug, Default)]
pub struct IdIndex<'b> {
    behavior: Option<&'b Behavior>,
    /// State machine -> its states (`hkbStateMachineStateInfo`). e.g. `#0100` -> `[#0101, #0102]`
    states_of: HashMap<&'b str, Vec<&'b str>>,
    /// State -> its state machine. e.g. `#0101` -> `#0100`
    machine_of: HashMap<&'b str, &'b str>,
    /// State -> its `stateId` and the line of it. e.g. `#0101` -> `(0, 7)`
    state_ids: HashMap<&'b str, (i64, usize)>,
    /// Array of transitions -> the state machine whose states they go to, and the state they leave
    /// (`None`: the wildcards of the state machine). e.g. `#0091` -> `(#0100, Some(#0101))`
    transitions: HashMap<&'b str, (&'b str, Option<&'b str>)>,
    /// The names object of the behavior: `(id, text)`. e.g. `#0083`
    names: Option<(&'b str, &'b str)>,
    /// How many events / variables the behavior has: its own and its referenced mods'. `None`: it
    /// has no names object to count them in.
    events: Option<usize>,
    variables: Option<usize>,
}

impl<'b> IdIndex<'b> {
    /// Reads the state machines and the names of `behavior`.
    pub fn new(behavior: &'b Behavior) -> Self {
        let mut index = Self { behavior: Some(behavior), ..Self::default() };
        let mut transitions_of_state: Vec<(&'b str, &'b str)> = Vec::new();
        let mut names: Option<(&'b str, &'b str)> = None;
        for object in behavior.texts() {
            match object.class {
                STRING_DATA if !behavior.is_ref(object.id) => names = Some((object.id, object.text)),
                STATE_MACHINE => {
                    let states = pointers_of(object.text, "states");
                    for state in &states {
                        index.machine_of.insert(state, object.id);
                    }
                    index.states_of.insert(object.id, states);
                    for wildcards in pointers_of(object.text, "wildcardTransitions") {
                        index.transitions.insert(wildcards, (object.id, None));
                    }
                }
                STATE_INFO => {
                    if let Some(state_id) = number_of(object.text, "stateId") {
                        index.state_ids.insert(object.id, state_id);
                    }
                    let arrays = pointers_of(object.text, "transitions");
                    transitions_of_state.extend(arrays.into_iter().map(|array| (array, object.id)));
                }
                _ => {}
            }
        }
        for (array, state) in transitions_of_state {
            if let Some(&machine) = index.machine_of.get(state) {
                index.transitions.insert(array, (machine, Some(state)));
            }
        }

        index.names = names;
        if let Some((_, names)) = names {
            let count = |field: &str, of_refs: usize| Some(nemesis_diff::vars::names(names, field).len() + of_refs);
            index.events = count("eventNames", behavior.ref_events.len());
            index.variables = count("variableNames", behavior.ref_variables.len());
        }
        index
    }

    /// The objects whose ids may be right or wrong once `id` changed: the states and the transitions
    /// of the state machine of `id` (a state machine, or a state of it).
    ///
    /// e.g. `#0101` (a state of `#0100`) -> `#0100`, `#0102`, `#0091`, `#0090`
    pub fn related(&self, id: &str) -> Vec<&'b str> {
        let machine = match self.machine_of.get(id) {
            Some(&machine) => machine,
            None => match self.states_of.get_key_value(id) {
                Some((&machine, _)) => machine,
                None => return Vec::new(),
            },
        };
        let states = self.states_of.get(machine).into_iter().flatten().copied();
        let arrays = self.transitions.iter().filter(|(_, (of, _))| *of == machine).map(|(&array, _)| array);
        std::iter::once(machine).chain(states).chain(arrays).filter(|other| *other != id).collect()
    }

    /// The state of `machine` whose `stateId` is `state_id`, but `other_than`.
    fn state_with(&self, states: &[&'b str], state_id: i64, other_than: &str) -> Option<&'b str> {
        let has_id = |state: &&&'b str| {
            **state != other_than && self.state_ids.get(**state).map(|(id, _)| *id) == Some(state_id)
        };
        states.iter().find(has_id).copied()
    }

    /// The state machine `state` goes into, if its generator is one (maybe wrapped). e.g. a state
    /// whose generator is `#0200 hkbStateMachine` -> `#0200`
    fn nested_machine(&self, state: &str) -> Option<&'b str> {
        let behavior = self.behavior?;
        let mut field = "generator";
        let mut at = behavior.object_text(state)?;
        for _ in 0..=GENERATOR_WRAPPERS.len() {
            let next = *pointers_of(at.text, field).first()?;
            let object = behavior.object_text(next)?;
            if object.class == STATE_MACHINE {
                return Some(object.id);
            }
            field = GENERATOR_WRAPPERS.iter().find(|(class, _)| *class == object.class)?.1;
            at = object;
        }
        None
    }
}

/// The id of an object's text, from its first line. e.g. `<hkobject name="#0091" ..>` -> `#0091`
fn object_id(text: &str) -> Option<&str> {
    let (_, rest) = text.lines().next()?.split_once("name=\"")?;
    rest.split_once('"').map(|(id, _)| id)
}

/// The problems of the ids of the object `id` whose text is `text` (it may be newer than the
/// behavior's). See the module docs.
pub fn id_diagnostics(text: &str, id: &str, index: &IdIndex<'_>) -> Vec<Diagnostic> {
    let fields = id_fields(text);
    let mut problems = Vec::new();
    // The `toStateId` of the transition being read: its `toNestedStateId` is in its state.
    let mut to_state: Option<i64> = None;
    for field in &fields {
        if field.kind == IdKind::ToState {
            to_state = Some(field.value);
        }
        if let Some(message) = id_problem(field, (text, id), index, to_state) {
            let columns = field.columns.clone();
            problems.push(Diagnostic { line: field.line, columns, severity: severity_of(field.kind), message });
        }
    }
    problems
}

/// How bad a wrong id of `kind` is, as the game takes it:
///
/// | kind                                   | of no state             | so      |
/// | -------------------------------------- | ----------------------- | ------- |
/// | `toStateId`                            | the game crashes once the transition is taken (seen with FNIS) | error   |
/// | `stateId` also of another state        | which one is meant is not known | error   |
/// | `startStateId`, `to/fromNestedStateId` | taken all the same: vanilla (`toNestedStateId` `0`, 47 times) and mods (BFCO's `#bfco$231`: `startStateId` `0`, states `1`..`3`) have them, and work | warning |
/// | an event / a variable past the names   | names nothing           | error   |
const fn severity_of(kind: IdKind) -> Severity {
    match kind {
        IdKind::StartState | IdKind::ToNestedState | IdKind::FromNestedState => Severity::Warning,
        IdKind::State | IdKind::ToState | IdKind::Event | IdKind::Variable => Severity::Error,
    }
}

/// What is wrong with `field` of the object `(text, id)`, if anything.
fn id_problem(field: &IdField, (text, id): (&str, &str), index: &IdIndex<'_>, to_state: Option<i64>) -> Option<String> {
    let value = field.value;
    if value < 0 {
        return None; // none: `-1`
    }
    let no_state = |machine: &str| format!("The state machine `{machine}` has no state of stateId `{value}`.");
    match field.kind {
        IdKind::State => {
            let machine = index.machine_of.get(id)?;
            let other = index.state_with(index.states_of.get(machine)?, value, id)?;
            Some(format!("stateId `{value}` is also the state `{other}`'s, in the state machine `{machine}`."))
        }
        IdKind::StartState => {
            // Its own states, as its text has them now.
            let states = pointers_of(text, "states");
            (!states.is_empty() && index.state_with(&states, value, "").is_none()).then(|| no_state(id))
        }
        IdKind::ToState => {
            let (machine, _) = index.transitions.get(id)?;
            let missing = index.state_with(index.states_of.get(machine)?, value, "").is_none();
            missing.then(|| format!("{} The game crashes when this transition is taken.", no_state(machine)))
        }
        // The templates leave many at `0` with the flag set, whatever the nested states are: it is
        // taken as not set.
        IdKind::ToNestedState | IdKind::FromNestedState if value == 0 => None,
        IdKind::ToNestedState => {
            let (machine, _) = index.transitions.get(id)?;
            let state = index.state_with(index.states_of.get(machine)?, to_state?, "")?;
            let nested = index.nested_machine(state)?;
            index.state_with(index.states_of.get(nested)?, value, "").is_none().then(|| no_state(nested))
        }
        IdKind::FromNestedState => {
            let (_, from) = index.transitions.get(id)?;
            let nested = index.nested_machine((*from)?)?;
            index.state_with(index.states_of.get(nested)?, value, "").is_none().then(|| no_state(nested))
        }
        IdKind::Event => {
            let events = index.events?;
            (usize::try_from(value).ok()? >= events).then(|| format!("No event #{value}: the behavior has {events}."))
        }
        IdKind::Variable => {
            let variables = index.variables?;
            (usize::try_from(value).ok()? >= variables)
                .then(|| format!("No variable #{value}: the behavior has {variables}."))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::workspace::ADSF;

    /// A behavior of only new objects.
    pub(super) fn behavior_of(objects: Vec<(String, String)>) -> Behavior {
        let mut behavior = Behavior::open(&Path::new("meshes").join(format!("{ADSF}.bin"))).unwrap();
        behavior.added.extend(objects);
        behavior
    }

    /// `(id, text)` of an object.
    pub(super) fn object(id: &str, class: &str, fields: &str) -> (String, String) {
        (id.to_owned(), format!("<hkobject name=\"{id}\" class=\"{class}\" signature=\"0x1\">\n{fields}</hkobject>\n"))
    }

    pub(super) fn state(id: &str, state_id: i64, more: &str) -> (String, String) {
        object(id, STATE_INFO, &format!("<hkparam name=\"stateId\">{state_id}</hkparam>\n{more}"))
    }

    pub(super) fn machine(id: &str, states: &str, more: &str) -> (String, String) {
        let states = format!("<hkparam name=\"states\" numelements=\"2\">\n{states}\n</hkparam>\n");
        object(id, STATE_MACHINE, &format!("<hkparam name=\"startStateId\">0</hkparam>\n{states}{more}"))
    }

    pub(super) fn transitions(id: &str, to: &str) -> (String, String) {
        let array =
            format!("<hkparam name=\"transitions\" numelements=\"1\">\n<hkobject>\n{to}</hkobject>\n</hkparam>\n");
        object(id, "hkbStateMachineTransitionInfoArray", &array)
    }

    /// `#m` with the states `#a` (0, its transitions `#t`) and `#b` (1), its wildcards `#w`; `#n` in
    /// `#b` (through `#g`) with `#c` (5). `to`: the fields of each transition.
    pub(super) fn machines(to: &str) -> Behavior {
        behavior_of(vec![
            machine("#m", "#a\n#b", "<hkparam name=\"wildcardTransitions\">#w</hkparam>\n"),
            state("#a", 0, "<hkparam name=\"transitions\">#t</hkparam>\n"),
            state("#b", 1, "<hkparam name=\"generator\">#g</hkparam>\n"),
            object("#g", "hkbModifierGenerator", "<hkparam name=\"generator\">#n</hkparam>\n"),
            machine("#n", "#c", ""),
            state("#c", 5, ""),
            transitions("#t", to),
            transitions("#w", to),
        ])
    }

    fn messages(behavior: &Behavior, id: &str) -> Vec<String> {
        let index = IdIndex::new(behavior);
        let text = behavior.text(id).unwrap();
        id_diagnostics(text, id, &index).into_iter().map(|problem| problem.message).collect()
    }

    #[test]
    fn a_transition_goes_to_a_state_of_its_state_machine() {
        let to = |state_id: i64| format!("<hkparam name=\"toStateId\">{state_id}</hkparam>\n");
        let fine = machines(&to(1));
        assert!(messages(&fine, "#t").is_empty());
        assert!(messages(&fine, "#w").is_empty(), "a wildcard, of the same state machine");

        let wrong = machines(&to(5));
        let crash =
            "The state machine `#m` has no state of stateId `5`. The game crashes when this transition is taken.";
        assert_eq!(messages(&wrong, "#t"), [crash]);
        assert_eq!(messages(&wrong, "#w").len(), 1);
    }

    #[test]
    fn a_nested_state_is_of_the_state_machine_its_state_goes_into() {
        let to = |nested: i64| {
            let flags = "<hkparam name=\"flags\">FLAG_TO_NESTED_STATE_ID_IS_VALID</hkparam>\n";
            format!(
                "<hkparam name=\"toStateId\">1</hkparam>\n<hkparam name=\"toNestedStateId\">{nested}</hkparam>\n{flags}"
            )
        };
        assert!(messages(&machines(&to(5)), "#t").is_empty(), "`#c` of `#n`, through `#g`");
        assert_eq!(messages(&machines(&to(4)), "#t"), ["The state machine `#n` has no state of stateId `4`."]);
        assert!(messages(&machines(&to(0)), "#t").is_empty(), "`0`: not set, as the templates have it");
    }

    #[test]
    fn a_state_id_is_one_state_s_in_its_state_machine() {
        let mut behavior = machines("");
        assert!(messages(&behavior, "#a").is_empty());
        behavior.added.extend([state("#b", 0, "")]);
        assert_eq!(messages(&behavior, "#a"), ["stateId `0` is also the state `#b`'s, in the state machine `#m`."]);
        assert!(messages(&behavior, "#c").is_empty(), "another state machine");

        behavior.added.extend([state("#c", 9, "")]);
        assert_eq!(messages(&behavior, "#n"), ["The state machine `#n` has no state of stateId `0`."], "startStateId");
        let index = IdIndex::new(&behavior);
        let start = id_diagnostics(behavior.text("#n").unwrap(), "#n", &index);
        assert_eq!(start[0].severity, Severity::Warning, "mods leave it at `0`, and work");
    }

    /// The templates are right: nothing of them is told. Reads every template behavior, so slow.
    #[test]
    #[ignore = "reads every template"]
    fn the_templates_have_no_problem() {
        fn files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            for path in std::fs::read_dir(dir).into_iter().flatten().flatten().map(|entry| entry.path()) {
                if path.is_dir() { files(&path, out) } else { out.push(path) }
            }
        }

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../resource/assets/templates/meshes");
        let mut paths = Vec::new();
        files(&root, &mut paths);
        let mut checked = 0;
        for path in paths {
            let is_behavior = path.extension().is_some_and(|ext| ext == "bin")
                && path.parent().is_some_and(|dir| dir.ends_with("behaviors") || dir.ends_with("characters"));
            let Some(behavior) = is_behavior.then(|| Behavior::open(&path).ok()).flatten() else { continue };

            let index = IdIndex::new(&behavior);
            for object in behavior.texts() {
                let problems = id_diagnostics(object.text, object.id, &index);
                assert!(problems.is_empty(), "{} {}: {problems:?}", path.display(), object.id);
            }
            checked += 1;
        }
        assert!(checked > 0);
    }

    #[test]
    fn a_state_s_change_concerns_its_state_machine() {
        let behavior = machines("");
        let index = IdIndex::new(&behavior);
        let mut related = index.related("#a");
        related.sort_unstable();
        assert_eq!(related, ["#b", "#m", "#t", "#w"]);
        assert!(index.related("#g").is_empty(), "no state, no state machine");
    }

    #[test]
    fn an_event_is_one_the_behavior_has() {
        let names = "<hkparam name=\"eventNames\" numelements=\"2\">\n<hkcstring>a</hkcstring>\n<hkcstring>b</hkcstring>\n</hkparam>\n";
        let event = |id: i64| {
            object("#e", "hkbEventDrivenModifier", &format!("<hkparam name=\"activateEventId\">{id}</hkparam>\n"))
        };
        let mut behavior = behavior_of(vec![object("#s", STRING_DATA, names), event(1)]);
        assert!(messages(&behavior, "#e").is_empty());

        behavior.added.extend([event(2)]);
        assert_eq!(messages(&behavior, "#e"), ["No event #2: the behavior has 2."]);
        behavior.ref_events.push("c".to_owned());
        assert!(messages(&behavior, "#e").is_empty(), "a referenced mod's");
    }
}
