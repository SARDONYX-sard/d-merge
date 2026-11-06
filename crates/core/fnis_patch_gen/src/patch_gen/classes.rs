//! The Havok classes that the FNIS patches add again and again.
//!
//! Each of them has many fields, of which few differ. So here they are written once, and the
//! generators only say what is special about theirs: what is left out is what FNIS always writes.
//!
//! - A value like `#Mod$1` or `#0000` is the `name` attribute of another class in the XML: a pointer.
//!   `#0000` is the null pointer.
//! - See `patch_gen/snapshots` for what comes out.
use std::borrow::Cow;

use json_patch::{JsonPath, ValueWithPriority, json_value};
use simd_json::BorrowedValue;

use super::new_class_patch;

/// The null pointer of a Havok XML.
pub(super) const NULL: &str = "#0000";

/// `hkbStateMachineStateInfo`: a state of a state machine, which runs `generator` while it is active.
#[derive(Debug)]
pub(super) struct StateInfo<'a> {
    pub index: &'a str,
    pub generator: &'a str,
    pub name: Cow<'a, str>,
    /// What transitions refer to. Unique in its state machine.
    pub state_id: i32,
    /// `hkbStateMachineEventPropertyArray`: the events sent when the state is entered.
    pub enter_notify_events: &'a str,
    /// `hkbStateMachineEventPropertyArray`: the events sent when the state is left.
    pub exit_notify_events: &'a str,
    /// `hkbStateMachineTransitionInfoArray`
    pub transitions: &'a str,
}

impl Default for StateInfo<'_> {
    fn default() -> Self {
        Self {
            index: NULL,
            generator: NULL,
            name: Cow::Borrowed(""),
            state_id: 0,
            enter_notify_events: NULL,
            exit_notify_events: NULL,
            transitions: NULL,
        }
    }
}

impl<'a> StateInfo<'a> {
    #[must_use]
    pub(super) fn into_patch(self, priority: usize) -> (JsonPath<'a>, ValueWithPriority<'a>) {
        let Self { index, generator, name, state_id, enter_notify_events, exit_notify_events, transitions } = self;
        new_class_patch(
            index,
            "hkbStateMachineStateInfo",
            priority,
            json_value!({
                "__ptr": index,
                "variableBindingSet": NULL,
                "listeners": [],
                "enterNotifyEvents": enter_notify_events,
                "exitNotifyEvents": exit_notify_events,
                "transitions": transitions,
                "generator": generator,
                "name": name,
                "stateId": state_id,
                "probability": 1.0,
                "enable": true
            }),
        )
    }
}

/// `hkbStateMachine`: goes from state to state of `states`, by events.
#[derive(Debug)]
pub(super) struct StateMachine<'a> {
    pub index: &'a str,
    pub name: Cow<'a, str>,
    /// The `hkbStateMachineStateInfo`s.
    pub states: Vec<&'a str>,
    /// `hkbVariableBindingSet`
    pub variable_binding_set: &'a str,
    pub start_state_id: i32,
    /// The event that goes to a random state. `-1`: none.
    pub random_transition_event_id: i32,
    /// `hkbStateMachineTransitionInfoArray`: transitions from any state.
    pub wildcard_transitions: &'a str,
}

impl Default for StateMachine<'_> {
    fn default() -> Self {
        Self {
            index: NULL,
            name: Cow::Borrowed(""),
            states: Vec::new(),
            variable_binding_set: NULL,
            start_state_id: 0,
            random_transition_event_id: -1,
            wildcard_transitions: NULL,
        }
    }
}

impl<'a> StateMachine<'a> {
    #[must_use]
    pub(super) fn into_patch(self, priority: usize) -> (JsonPath<'a>, ValueWithPriority<'a>) {
        let Self {
            index,
            name,
            states,
            variable_binding_set,
            start_state_id,
            random_transition_event_id,
            wildcard_transitions,
        } = self;
        new_class_patch(
            index,
            "hkbStateMachine",
            priority,
            json_value!({
                "__ptr": index,
                "variableBindingSet": variable_binding_set,
                "userData": 0,
                "name": name,
                "eventToSendWhenStateOrTransitionChanges": {
                    "id": -1,
                    "payload": NULL
                },
                "startStateChooser": NULL,
                "startStateId": start_state_id,
                "returnToPreviousStateEventId": -1,
                "randomTransitionEventId": random_transition_event_id,
                "transitionToNextHigherStateEventId": -1,
                "transitionToNextLowerStateEventId": -1,
                "syncVariableIndex": -1,
                "wrapAroundStateId": false,
                "maxSimultaneousTransitions": 32,
                "startStateMode": "START_STATE_MODE_DEFAULT",
                "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                "states": states,
                "wildcardTransitions": wildcard_transitions
            }),
        )
    }
}

/// `hkbClipGenerator`: plays an animation file.
#[derive(Debug)]
pub(super) struct ClipGenerator<'a> {
    pub index: &'a str,
    pub name: Cow<'a, str>,
    /// e.g. `Animations\FNISZoo\idle.hkx`
    pub animation_name: Cow<'a, str>,
    /// `hkbClipTriggerArray`: the events at certain times of the animation.
    pub triggers: &'a str,
    /// `MODE_SINGLE_PLAY` or `MODE_LOOPING`
    pub mode: &'static str,
}

impl<'a> ClipGenerator<'a> {
    #[must_use]
    pub(super) fn into_patch(self, priority: usize) -> (JsonPath<'a>, ValueWithPriority<'a>) {
        let Self { index, name, animation_name, triggers, mode } = self;
        new_class_patch(
            index,
            "hkbClipGenerator",
            priority,
            json_value!({
                "__ptr": index,
                "variableBindingSet": NULL,
                "userData": 0,
                "name": name,
                "animationName": animation_name,
                "triggers": triggers,
                "cropStartAmountLocalTime": 0.0,
                "cropEndAmountLocalTime": 0.0,
                "startTime": 0.0,
                "playbackSpeed": 1.0,
                "enforcedDuration": 0.0,
                "userControlledTimeFraction": 0.0,
                "animationBindingIndex": -1,
                "mode": mode,
                "flags": 0
            }),
        )
    }
}

/// `hkbModifierGenerator`: runs `modifier` over what `generator` makes.
#[derive(Debug)]
pub(super) struct ModifierGenerator<'a> {
    pub index: &'a str,
    pub name: Cow<'a, str>,
    pub modifier: &'a str,
    pub generator: &'a str,
}

impl<'a> ModifierGenerator<'a> {
    #[must_use]
    pub(super) fn into_patch(self, priority: usize) -> (JsonPath<'a>, ValueWithPriority<'a>) {
        let Self { index, name, modifier, generator } = self;
        new_class_patch(
            index,
            "hkbModifierGenerator",
            priority,
            json_value!({
                "__ptr": index,
                "variableBindingSet": NULL,
                "userData": 1,
                "name": name,
                "modifier": modifier,
                "generator": generator
            }),
        )
    }
}

/// `hkbVariableBindingSet`: `bindings` tie fields of a class to behavior variables.
#[must_use]
pub(super) fn new_variable_binding_set<'a>(
    index: &'a str,
    priority: usize,
    bindings: BorrowedValue<'a>,
) -> (JsonPath<'a>, ValueWithPriority<'a>) {
    new_class_patch(
        index,
        "hkbVariableBindingSet",
        priority,
        json_value!({
            "__ptr": index,
            "bindings": bindings,
            "indexOfBindingToEnable": -1
        }),
    )
}

/// `hkbStringEventPayload`: the name of an animation object, sent along with an event.
///
/// e.g. `AnimObjectSword`
#[must_use]
pub(super) fn new_anim_object<'a>(
    index: &'a str,
    name: &'a str,
    priority: usize,
) -> (JsonPath<'a>, ValueWithPriority<'a>) {
    new_class_patch(
        index,
        "hkbStringEventPayload",
        priority,
        json_value!({
            "__ptr": index,
            "data": name, // StringPtr
        }),
    )
}

/// `hkbClipTriggerArray`: the events at certain times of an animation.
#[must_use]
pub(super) fn new_clip_trigger_array<'a>(
    index: &'a str,
    priority: usize,
    triggers: BorrowedValue<'a>,
) -> (JsonPath<'a>, ValueWithPriority<'a>) {
    new_class_patch(
        index,
        "hkbClipTriggerArray",
        priority,
        json_value!({
            "__ptr": index,
            "triggers": triggers
        }),
    )
}

/// `hkbStateMachineEventPropertyArray`: the events a state sends.
#[must_use]
pub(super) fn new_event_array<'a>(
    index: &'a str,
    priority: usize,
    events: BorrowedValue<'a>,
) -> (JsonPath<'a>, ValueWithPriority<'a>) {
    new_class_patch(
        index,
        "hkbStateMachineEventPropertyArray",
        priority,
        json_value!({
            "__ptr": index,
            "events": events
        }),
    )
}
