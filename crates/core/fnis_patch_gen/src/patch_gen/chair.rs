//! To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
//!
//! # How to Read the FNIS Template for Chair
use std::borrow::Cow;

use fnis_list::patterns::chair::FNISChairAnimation;
use json_patch::{ValueWithPriority, json_path, json_value};
use rayon::prelude::*;

use crate::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        JsonPatchPairs,
        classes::{
            ClipGenerator, ModifierGenerator, StateInfo, StateMachine, new_anim_object, new_clip_trigger_array,
            new_event_array, new_variable_binding_set,
        },
        furniture::one_anim::new_push_values_seq_patch,
        global::mt_behavior::FNIS_AA_MT_AUTO_GEN_5222,
        kill_move::calculate_hash,
        new_class_patch, new_push_events_seq_patch, new_seq_push,
    },
};

/// Creates JSON patches for a single chair animation, depending on its phase.
///
/// - `has_any_anim_obj`: Whether any furniture sequence contains an animation object
///
/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub(super) fn new_chair_patches<'a>(
    chair: &FNISChairAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [&'a str; 19] = core::array::from_fn(|_| owned_data.next_class_name_attribute());

    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;

    // NOTE: Must be 1 file unique ID
    let class_index_0_id = calculate_hash(class_indexes[0]);
    let [anim_file_0, anim_file_1, anim_file_2, anim_file_3, anim_file_4] = build_anim_files(namespace, chair);

    let start_animation = &chair.start;
    let start_event = start_animation.anim_event;

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.extend({
        let mut events = Vec::with_capacity(1 + chair.start.flag_set.triggers.len());
        events.push(start_event.into());
        events.extend(chair.start.flag_set.triggers.iter().map(|trigger| trigger.event.into()));
        new_push_events_seq_patch(events, "#0083", "#0085", priority)
    });
    if !start_animation.flag_set.anim_vars.is_empty() {
        seq_patches.par_extend(new_push_values_seq_patch(
            &start_animation.flag_set.anim_vars,
            "#0083",
            "#0084",
            "#0085",
            priority,
        ));
    }

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let mut anim_object_indexes = Vec::with_capacity(start_animation.anim_objects.len());
    one_patches.extend(start_animation.anim_objects.iter().map(|name| {
        let new_anim_object_index = owned_data.next_class_name_attribute();
        anim_object_indexes.push(new_anim_object_index);

        // One anim object
        new_anim_object(new_anim_object_index, name, priority)
    }));

    // $RI
    one_patches.push(
        StateInfo {
            index: class_indexes[0],
            enter_notify_events: class_indexes[1],
            exit_notify_events: "#1731",
            transitions: "#1730",
            generator: class_indexes[2],
            name: format!("FNISChairIdleState{class_index_0_id}").into(),
            state_id: class_index_0_id, // $101/1$
        }
        .into_patch(priority),
    );

    // $RI+1
    one_patches.push(new_event_property_array_ri1(class_indexes[1], &anim_object_indexes, priority));

    // $RI+2
    one_patches.push(
        StateMachine {
            index: class_indexes[2],
            variable_binding_set: class_indexes[3],
            name: format!("FNISChairIdleBehavior{class_index_0_id}").into(), // FNISChairIdleBehavior$1/1$
            states: vec![class_indexes[4], class_indexes[7], class_indexes[15]],
            wildcard_transitions: "#1720",
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+3
    one_patches.push(new_variable_binding_set(
        class_indexes[3],
        priority,
        json_value!([
            {
                "memberPath": "isActive",
                "variableIndex": 60, // bTalkableWithItem
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // $RI+4
    one_patches.push(
        StateInfo {
            index: class_indexes[4],
            transitions: "#1726",
            generator: class_indexes[5],
            name: format!("FNISChairIdleStart{class_index_0_id}").into(), // FNISChairIdleStart$1/1$
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+5
    one_patches.push(
        ClipGenerator {
            index: class_indexes[5],
            name: format!("FNISChairIdleStart{class_index_0_id}").into(), // FNISChairIdleStart$1/1$
            animation_name: anim_file_0.into(),                           // Animations\\$chg-4$
            triggers: class_indexes[6],
            mode: "MODE_SINGLE_PLAY",
        }
        .into_patch(priority),
    );

    // $RI+6
    one_patches.push({
        let mut triggers = Vec::with_capacity(1 + anim_object_indexes.len());
        triggers.push(json_value!({
            "localTime": -0.2,
            "event": {
                "id": 91, // ChairNextClip
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));

        triggers.extend(anim_object_indexes.iter().map(|anim_object_index| {
            json_value!({
                "localTime": 1.0,
                "event": {
                    "id": 394, // AnimObjDraw
                    "payload": *anim_object_index,
                },
                "relativeToEndOfClip": false,
                "acyclic": false,
                "isAnnotation": false
            })
        }));

        new_clip_trigger_array(class_indexes[6], priority, json_value!(triggers))
    });

    // $RI+7
    one_patches.push(
        StateInfo {
            index: class_indexes[7],
            generator: class_indexes[8],
            name: format!("FNISChairIdle{class_index_0_id}").into(), // FNISChairIdle$1/1$
            state_id: 1,
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+8
    one_patches.push(
        StateMachine {
            index: class_indexes[8],
            name: format!("FNISChairIdleBehavior{class_index_0_id}").into(), // FNISChairIdleBehavior$1/1$
            random_transition_event_id: 91,                                  // ChairNextClip
            states: vec![class_indexes[9], class_indexes[11], class_indexes[13]],
            wildcard_transitions: "#1117",
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+9
    one_patches.push(
        StateInfo {
            index: class_indexes[9],
            generator: class_indexes[10],
            name: format!("FNISChairIdleBase{class_index_0_id}").into(), // FNISChairIdleBase$1/1$
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+10
    one_patches.push(
        ClipGenerator {
            index: class_indexes[10],
            name: format!("FNISChairIdleBase{class_index_0_id}").into(), // FNISChairIdleBase$1/1$
            animation_name: anim_file_1.into(),                          // Animations\\$chg-3$
            triggers: "#1130",
            mode: "MODE_LOOPING",
        }
        .into_patch(priority),
    );

    // $RI+11
    one_patches.push(
        StateInfo {
            index: class_indexes[11],
            enter_notify_events: "#1127",
            exit_notify_events: "#1126",
            generator: class_indexes[12],
            name: format!("FNISChairIdleVar1_{class_index_0_id}").into(), // FNISChairIdleVar1_$1/1$
            state_id: 1,
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+12
    one_patches.push(
        ClipGenerator {
            index: class_indexes[12],
            name: format!("FNISChairIdleVar1_{class_index_0_id}").into(), // FNISChairIdleVar1_$1/1$
            animation_name: anim_file_2.into(),                           // Animations\\$chg-2$
            triggers: FNIS_AA_MT_AUTO_GEN_5222,
            mode: "MODE_LOOPING",
        }
        .into_patch(priority),
    );

    // $RI+13
    one_patches.push(
        StateInfo {
            index: class_indexes[13],
            enter_notify_events: "#1127",
            exit_notify_events: "#1126",
            generator: class_indexes[14],
            name: format!("FNISChairIdleVar2_{class_index_0_id}").into(), // FNISChairIdleVar2_$1/1$
            state_id: 2,
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+14
    one_patches.push(
        ClipGenerator {
            index: class_indexes[14],
            name: format!("FNISChairIdleVar2_{class_index_0_id}").into(), // FNISChairIdleVar2_$1/1$
            animation_name: anim_file_3.into(),                           // Animations\\$chg-1$
            triggers: FNIS_AA_MT_AUTO_GEN_5222,
            mode: "MODE_LOOPING",
        }
        .into_patch(priority),
    );

    // $RI+15
    one_patches.push(
        StateInfo {
            index: class_indexes[15],
            transitions: "#1721",
            generator: class_indexes[16],
            name: format!("FNISChairTalkingState{class_index_0_id}").into(), // FNISChairTalkingState$1/1$
            state_id: 2,
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+16
    one_patches.push(
        ModifierGenerator {
            index: class_indexes[16],
            name: format!("FNISChairIdle_MG{class_index_0_id}").into(), // FNISChairIdle_MG$1/1$
            modifier: class_indexes[17],
            generator: class_indexes[18],
        }
        .into_patch(priority),
    );

    // $RI+17
    one_patches.push(new_class_patch(
        class_indexes[17],
        "BSEventEveryNEventsModifier",
        priority,
        json_value!({
            "__ptr": class_indexes[17],
            "variableBindingSet": "#0000",
            "userData": 3,
            "name": format!("BSEventEveryNEventsMod_5and2_{class_index_0_id}"), // BSEventEveryNEventsMod_5and2_$1/1$
            "enable": true,
            "eventToCheckFor": {
                "id": 168, // CountDownTick
                "payload": "#0000"
            },
            "eventToSend": {
                "id": 132, // 00NextClip
                "payload": "#0000"
            },
            "numberOfEventsBeforeSend": 5,
            "minimumNumberOfEventsBeforeSend": 2,
            "randomizeNumberOfEvents": true,
        }),
    ));

    // $RI+18
    one_patches.push(
        ClipGenerator {
            index: class_indexes[18],
            name: format!("FNISChairTalking{class_index_0_id}").into(), // FNISChairTalking$1/1$
            animation_name: anim_file_4.into(),                         // Animations\\$chg-3$
            triggers: "#1114",
            mode: "MODE_LOOPING",
        }
        .into_patch(priority),
    );

    // Push the first animation for chair seq
    seq_patches.push({
        new_seq_push(
            json_patch::json_path!["#1227", "hkbStateMachineTransitionInfoArray", "transitions"],
            priority,
            json_value!([{
                "triggerInterval": {
                    "enterEventId": -1,
                    "exitEventId": -1,
                    "enterTime": 0.0,
                    "exitTime": 0.0
                },
                "initiateInterval": {
                    "enterEventId": -1,
                    "exitEventId": -1,
                    "enterTime": 0.0,
                    "exitTime": 0.0
                },
                "transition": "#0091",
                "condition": "#0000",
                "eventId": format!("$eventID[{start_event}]$"), // eventId is Nemesis variable
                "toStateId": class_index_0_id, // Must match root_state
                "fromNestedStateId": 0,
                "toNestedStateId": 0,
                "priority": 0,
                "flags": "FLAG_DISABLE_CONDITION"
            }]),
        )
    });

    seq_patches.push(new_seq_push(
        json_path!["#1755", "hkbStateMachine", "states"],
        priority,
        json_value!([class_indexes[0]]),
    ));

    (one_patches, seq_patches)
}

#[must_use]
fn new_event_property_array_ri1<'a>(
    class_index: &'a str,
    anim_object_indexes: &[&'a str],
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let events: Vec<_> = anim_object_indexes
        .iter()
        .map(|anim_object_index| {
            json_value!({
                "id": 393, // AnimObjLoad
                "payload": *anim_object_index // #$:AnimObj+&ao$
            })
        })
        .collect();

    new_event_array(class_index, priority, json_value!(events))
}

/// Builds exactly 5 animation file paths.
///
/// Missing sequenced animations reuse the last available animation.
///
/// - [playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&gist=88299202103d425264e60e69244cf9e9)
fn build_anim_files(namespace: &str, chair: &FNISChairAnimation) -> [String; 5] {
    let FNISChairAnimation { start, sequenced } = chair;
    let start_anim_file = start.anim_file;

    core::array::from_fn(|i| {
        let anim_file = match i {
            0 => &start_anim_file,
            i => sequenced
                .get(i - 1) // get(0), get(1), ...
                .or_else(|| sequenced.first()) // get(0)
                .unwrap_or(&start_anim_file),
        };

        format!("Animations\\{namespace}\\{anim_file}")
    })
}

#[cfg(test)]
mod tests {
    use fnis_list::combinator::{
        anim_types::FNISAnimType,
        flags::{FNISAnimFlagSet, FNISAnimFlags},
        fnis_animation::FNISAnimation,
    };

    use super::*;

    #[test]
    fn test_build_anim_files() {
        let chair = FNISChairAnimation {
            start: FNISAnimation {
                anim_type: FNISAnimType::Chair,
                flag_set: FNISAnimFlagSet {
                    flags: FNISAnimFlags::AnimObjects,
                    blend_time: None,
                    triggers: vec![],
                    anim_vars: vec![],
                },
                anim_event: "TestStartEvent",
                anim_file: "test_start.hkx",
                anim_objects: vec!["TestObject"],
                anim_vars: vec![],
                motions: vec![],
                rotations: vec![],
            },
            sequenced: vec!["test_base.hkx", "test_variant1.hkx", "test_variant2.hkx"],
        };

        let result = build_anim_files("TestNamespace", &chair);

        assert_eq!(
            result,
            [
                "Animations\\TestNamespace\\test_start.hkx",
                "Animations\\TestNamespace\\test_base.hkx",
                "Animations\\TestNamespace\\test_variant1.hkx",
                "Animations\\TestNamespace\\test_variant2.hkx",
                "Animations\\TestNamespace\\test_base.hkx",
            ]
        );
    }
}
