//! To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
//!
//! # How to Read the FNIS Template for Furniture
//! - Note: This is my speculation and not definitive.
//!
//! As background, FNIS_modders.pdf states Furniture requires at least four animations. Meaning it's a special sequence.
//! `-F`: Start of the Furniture syntax sequence. The statement where `fu` is used.
//! `-L`: End of the Furniture syntax sequence (Last).
use std::borrow::Cow;

use fnis_list::combinator::{
    Trigger,
    flags::{AnimVar, FNISAnimFlags},
    fnis_animation::FNISAnimation,
};
use json_patch::{ValueWithPriority, json_path, json_value};
use rayon::prelude::*;
use simd_json::borrowed::Value;

use crate::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        JsonPatchPairs,
        classes::{
            ClipGenerator, ModifierGenerator, StateInfo, new_anim_object, new_clip_trigger_array, new_event_array,
            new_variable_binding_set,
        },
        global::mt_behavior::FNIS_AA_MT_AUTO_GEN_5235,
        kill_move::calculate_hash,
        new_class_patch, new_push_events_seq_patch, new_seq_push,
    },
};

/// Represents the phase of a Furniture animation sequence as defined in FNIS.
///
/// According to *FNIS_modders.pdf*, Furniture animations require at least four
/// animations forming a special sequence:
///
/// FNIS TEMPLATE.txt block
/// - `-F`(First): Start of the Furniture syntax sequence
/// - (middle entries): Continuation of the Furniture sequence
/// - `-L`(Last): End of the Furniture syntax sequence
///
/// This enum is used to identify which phase of the sequence is currently being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FurniturePhase {
    /// The first phase of the Furniture animation sequence (`-F`).
    Start,

    /// A middle phase of the Furniture sequence.
    /// Contains an index (e.g., `Middle(1)`, `Middle(2)`).
    Middle(usize),

    /// The final phase of the Furniture animation sequence (`-L`).
    /// Contains an index
    End(usize),
}

/// Creates JSON patches for a single Furniture animation, depending on its phase.
///
/// - `has_any_anim_obj`: Whether any furniture sequence contains an animation object
///
/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub(super) fn new_furniture_one_anim_patches<'a>(
    animation: &FNISAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
    has_any_anim_obj: bool,
    current_phase: FurniturePhase,
    class_indexes: &[&'a str; 9],
    end_anim_state_id: i32,
    next_event_name: Option<&'a str>,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let flags = animation.flag_set.flags;
    let event_name = animation.anim_event;

    let is_empty_anim_vars = animation.flag_set.anim_vars.is_empty();

    // NOTE: Must be 1 file unique ID
    let class_index_0_id = calculate_hash(class_indexes[0]);

    let anim_file = format!("Animations\\{namespace}\\{}", animation.anim_file); // Animations\\$Foa$

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.extend({
        let events: Vec<Cow<'a, str>> = if matches!(current_phase, FurniturePhase::End(_)) {
            vec![event_name.into(), format!("{}_DONE", event_name).into()]
        } else {
            vec![event_name.into()]
        };

        new_push_events_seq_patch(events, "#0083", "#0085", priority)
    });
    if !is_empty_anim_vars {
        seq_patches.par_extend(new_push_values_seq_patch(
            &animation.flag_set.anim_vars,
            "#0083",
            "#0084",
            "#0085",
            priority,
        ));
    }

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let mut anim_object_indexes = Vec::with_capacity(animation.anim_objects.len());
    one_patches.extend(animation.anim_objects.iter().map(|name| {
        let new_anim_object_index = owned_data.next_class_name_attribute();
        anim_object_indexes.push(new_anim_object_index);

        // One anim object
        new_anim_object(new_anim_object_index, name, priority)
    }));

    // $RI
    one_patches.push({
        // NOTE: For some reason, everything except Start seems to be a null pointer(confirmed by checking the actual XML output from FNIS).
        // $-o+|#%RI+1%|h-|#%RI+1%|ac1|#%RI+1%|null$
        let enter_notify_events = if matches!(current_phase, FurniturePhase::Start)
            && (flags.contains(FNISAnimFlags::AnimObjects)
                || !flags.contains(FNISAnimFlags::HeadTracking)
                || flags.contains(FNISAnimFlags::AnimatedCameraSet))
        {
            class_indexes[1]
        } else {
            "#0000"
        };

        // NOTE: For some reason, the middle element of seq seems to be a null pointer. (confirmed by checking the actual XML output from FNIS)
        // $-o-|#%RI+2%|h+|#%RI+2%|F|#%RI+2%|ac0|#%RI+2%|null$
        let exit_notify_events = if !matches!(current_phase, FurniturePhase::Middle(_))
            && (!flags.contains(FNISAnimFlags::AnimObjects)
                || flags.contains(FNISAnimFlags::HeadTracking) // h+
                || matches!(current_phase, FurniturePhase::Start) // F
                || flags.contains(FNISAnimFlags::AnimatedCameraReset))
        {
            class_indexes[2]
        } else {
            "#0000"
        };

        // $-F|#%RI+3%|L|null|#%:FuTransExit%$
        let transition = if matches!(current_phase, FurniturePhase::End(_)) {
            "#0000"
        } else {
            // first phase $RI+3
            class_indexes[3] // `#%:FuTransExit` only matched two entries within mt_behavior, both pointing to RI+3.
        };

        // #$-AV|%RI+4%|%RI+7%$
        let generator = if is_empty_anim_vars { class_indexes[7] } else { class_indexes[4] };

        // Actually, since this comes in as 1, 2, 3... every sequence, it seems fine to reuse the same stateId.
        // one FNIS_List.txt
        // - fu syntax: 1, 2, 3
        // - fu syntax: 1, 2, 3
        let state_id = match current_phase {
            FurniturePhase::Start => 1,
            FurniturePhase::Middle(i) | FurniturePhase::End(i) => (i + 1) as i32,
        };

        StateInfo {
            index: class_indexes[0],
            enter_notify_events,
            exit_notify_events,
            transitions: transition,
            generator,
            name: event_name.into(),
            state_id, // $1+.fu$
        }
        .into_patch(priority)
    });
    one_patches.push({
        // #$:AnimObj+&ao$
        new_event_property_array_ri1(flags, anim_object_indexes.first().copied(), class_indexes[1], priority)
    });
    one_patches.push(new_event_property_array_ri2(flags, has_any_anim_obj, current_phase, class_indexes[2], priority));

    // #$RI+3$ hkbStateMachineTransitionInfoArray
    one_patches.push(new_class_patch(
        class_indexes[3],
        "hkbStateMachineTransitionInfoArray",
        priority,
        json_value!({
            "__ptr": class_indexes[3],
            "transitions": [
                {
                    "triggerInterval": {
                        "enterEventId": -1,
                        "exitEventId": -1,
                        "enterTime": 0.0,
                        "exitTime": 0.0,
                    },
                    "initiateInterval": {
                        "enterEventId": -1,
                        "exitEventId": -1,
                        "enterTime": 0.0,
                        "exitTime": 0.0,
                    },
                    "transition": FNIS_AA_MT_AUTO_GEN_5235, // #$:BlendTransition+&bl$
                    "condition": "#0000",
                    "eventId": 152, // IdleChairExitStart

                    // Last index of the sequence. (e.g., len 4 is 4. However,
                    // we use the hash of the last seq state class index here.)
                    "toStateId": end_anim_state_id, // $&fu$
                    "fromNestedStateId": 0,
                    "toNestedStateId": 0,
                    "priority": 0,
                    "flags": "FLAG_DISABLE_CONDITION",
                }
            ],
        }),
    ));

    // Skip to RI+7 if non use anim_vars
    if !is_empty_anim_vars {
        // RI+4
        one_patches.push(
            ModifierGenerator {
                index: class_indexes[4],
                name: "IdleMesmerizes_MG".into(),
                modifier: class_indexes[5],
                generator: class_indexes[7],
            }
            .into_patch(priority),
        );

        // #RI+5
        one_patches.push({
            let inverse_flags: [bool; 5] =
                core::array::from_fn(|i| animation.flag_set.anim_vars.get(i).is_some_and(|v| v.inverse));

            new_class_patch(
                class_indexes[5],
                "BSIsActiveModifier",
                priority,
                json_value!({
                    "__ptr": class_indexes[5],
                    "variableBindingSet": class_indexes[6],
                    "userData": 2,
                    "name": "bMesmerizeIsActive",
                    "enable": true,
                    "bIsActive0": false,
                    "bInvertActive0": inverse_flags[0],
                    "bIsActive1": false,
                    "bInvertActive1": inverse_flags[1],
                    "bIsActive2": false,
                    "bInvertActive2": inverse_flags[2],
                    "bIsActive3": false,
                    "bInvertActive3": inverse_flags[3],
                    "bIsActive4": false,
                    "bInvertActive4": inverse_flags[4],
                }),
            )
        });

        one_patches.push({
            let bindings: Vec<_> = animation
                .flag_set
                .anim_vars
                .iter()
                .enumerate()
                .map(|(i, var)| {
                    json_value!({
                        "memberPath": format!("bIsActive{i}"), // FIXME ?: Is this correct?
                        "variableIndex": format!("$variableID{}$", var.name),  // $&AVI$
                        "bitIndex": -1,
                        "bindingType": "BINDING_TYPE_VARIABLE"
                    })
                })
                .collect();

            new_variable_binding_set(class_indexes[6], priority, json_value!(bindings))
        });
    }

    // #$RI+7$  hkbClipGenerator
    one_patches.push({
        //  $-F|IdleBlessingKneelEnter|FNISCG_%BI%$
        //
        // The meaning of this template block is:
        // - first: `IdleBlessingKneelEnter`
        // - 2nd: `FNISCG_1`
        // - 3rd: `FNISCG_2`
        //
        // NOTE: Even when the fu syntax appeared twice within a single list, the names did not duplicate.
        // Therefore, it seems necessary to make them unique per file.
        let name = match current_phase {
            FurniturePhase::Start => Cow::Borrowed("IdleBlessingKneelEnter"),
            FurniturePhase::Middle(_) | FurniturePhase::End(_) => format!("FNISCG_{class_index_0_id}").into(),
        };

        // NOTE: `T` is probably has_triggers
        // $-L|#%RI+8%|a|#%RI+8%|T|#%RI+8%|null$
        let triggers_index = if matches!(current_phase, FurniturePhase::End(_))
            || flags.contains(FNISAnimFlags::Acyclic)
            || !animation.flag_set.triggers.is_empty()
        {
            class_indexes[8]
        } else {
            "#0000"
        };

        // $-a|MODE_SINGLE_PLAY|MODE_LOOPING$
        let mode = if flags.contains(FNISAnimFlags::Acyclic) { "MODE_SINGLE_PLAY" } else { "MODE_LOOPING" };

        ClipGenerator {
            index: class_indexes[7],
            name,
            animation_name: anim_file.into(), // Animations\\Ffu$
            triggers: triggers_index,
            mode,
        }
        .into_patch(priority)
    });
    one_patches.push({
        let triggers =
            new_values_from_triggers(flags, &animation.flag_set.triggers, event_name, next_event_name, current_phase);

        new_clip_trigger_array(class_indexes[8], priority, json_value!(triggers))
    });

    (one_patches, seq_patches)
}

#[must_use]
fn new_event_property_array_ri1<'a>(
    flags: FNISAnimFlags,
    anim_object_index: Option<&'a str>,
    class_index: &'a str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = Vec::new();

    // AnimObjects block (ao)
    if let Some(anim_obj_index) = anim_object_index {
        // 393: AnimObjLoad / 394: AnimObjDraw
        for id in [393, 394] {
            events.push(json_value!({
                "id": id,
                "payload": anim_obj_index
            }));
        }
    }

    // HeadTracking OFF block (h-)
    events.push(json_value!({
        "id": 20, // HeadTrackingOff
        "payload": "#0000"
    }));

    // AnimatedCameraSet block (-ac1)
    if flags.contains(FNISAnimFlags::AnimatedCameraSet) {
        events.push(json_value!({
            "id": 461, // StartAnimatedCamera
            "payload": "#1978"
        }));
    }

    new_event_array(class_index, priority, json_value!(events))
}

#[must_use]
fn new_event_property_array_ri2<'a>(
    flags: FNISAnimFlags,
    has_any_anim_obj: bool,
    current_phase: FurniturePhase,
    class_index: &'a str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = Vec::new();

    // `$-o-$`: Whether any furniture sequence contains an animation object
    if has_any_anim_obj {
        events.push(json_value!({
            "id": 165, // AnimObjectUnequip
            "payload": "#0000"
        }));
    }

    // HeadTracking ON block (-h+)
    if matches!(current_phase, FurniturePhase::End(_)) {
        events.push(json_value!({
            "id": 18, // HeadTrackingOn
            "payload": "#0000"
        }));
    }

    // $-F$: Furniture Start block
    if matches!(current_phase, FurniturePhase::Start) {
        events.push(json_value!({
            "id": 14, // IdleChairSitting
            "payload": "#0000"
        }));
    }

    // AnimatedCameraReset block (-ac0)
    if flags.contains(FNISAnimFlags::AnimatedCameraReset) {
        events.push(json_value!({
            "id": 462, // EndAnimatedCamera
            "payload": "#0000"
        }));
    }

    new_event_array(class_index, priority, json_value!(events))
}

#[must_use]
fn new_values_from_triggers<'a>(
    flags: FNISAnimFlags,
    triggers: &[Trigger<'a>],
    current_event_name: &'a str,
    next_event_name: Option<&'a str>,
    current_phase: FurniturePhase,
) -> Vec<Value<'a>> {
    let mut values: Vec<Value> = vec![];
    values.par_extend(triggers.par_iter().map(|Trigger { event, time }| {
        json_value!({
            "localTime": *time, // $&TT$
            "event": {
                "id": format!("$eventID[{event}]$"), // use Nemesis eventID variable. instead of $&TAE$
                "payload": "#0000"
            },
            // When there is no trigger time. The existence of this syntax may imply that the <time> element
            // in the T<event>/<time> syntax is optional.
            "relativeToEndOfClip": false, // FIXME: $&TT-$
            "acyclic": false,
            "isAnnotation": false
        })
    }));

    // `$-L$` <- When Furniture seq is last
    if matches!(current_phase, FurniturePhase::End(_)) {
        // "$AE1fug+&fug$", "5"
        values.push(json_value!({
            "localTime": -0.05,
            "event": {
                "id": 130, // IdleForceDefaultState
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));
        values.push(json_value!({
            "localTime": -0.2,
            "event": {
                "id": format!("$eventID[{current_event_name}_DONE]$"), // DONE AnimEvent: $AE1fug+&fug$ (Need: seq last event + _DONE suffix)
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));
        values.push(json_value!({
            "localTime": -0.2,
            "event": {
                "id": 5, // IdleFurnitureExit
                "payload": "#0000"
            },
            "relativeToEndOfClip": true,
            "acyclic": false,
            "isAnnotation": false
        }));
    }

    // NOTE: Furniture can only have the Acyclic flag set at the first or end.
    // And since next is required, this can only occur at the first.
    if flags.contains(FNISAnimFlags::Acyclic) {
        // If there is no next, then it is last.
        if let Some(next_event_name) = next_event_name {
            values.push(json_value!({
                "localTime": 0.0,
                "event": {
                    "id": format!("$eventID[{next_event_name}]$"), // next AnimEvent: $AE1fu+1/1$
                    "payload": "#0000"
                },
                "relativeToEndOfClip": false,
                "acyclic": false,
                "isAnnotation": false
            }));
        }
    }

    values
}

/// This variable likely needs to be registered below.
/// - `hkbBehaviorGraphStringData.variableNames`
/// - `hkbVariableValueSet.wordVariableValues`
/// - `hkbBehaviorGraphData.variableInfos`(as [i32])
pub(crate) fn new_push_values_seq_patch<'a>(
    values: &[AnimVar<'a>],
    string_data_index: &'static str,
    variable_index: &'static str,
    behavior_graph_index: &'static str,
    priority: usize,
) -> [(json_path::JsonPath<'a>, ValueWithPriority<'a>); 3] {
    [
        new_seq_push(
            json_path![string_data_index, "hkbBehaviorGraphStringData", "variableNames",],
            priority,
            values.iter().map(|value| value.name).collect(),
        ),
        new_seq_push(
            json_path![variable_index, "hkbVariableValueSet", "wordVariableValues"],
            priority,
            json_value!(
                values
                    .par_iter()
                    .map(|_| {
                        json_value!({
                            "value": 0
                        })
                    })
                    .collect::<Vec<_>>()
            ),
        ),
        new_seq_push(
            json_path![behavior_graph_index, "hkbBehaviorGraphData", "variableInfos",],
            priority,
            json_value!(
                values
                    .par_iter()
                    .map(|_| {
                        json_value!({
                            "role": {
                                "role": "ROLE_DEFAULT",
                                "flags": "0"
                            },
                            "type": "VARIABLE_TYPE_INT32"
                        })
                    })
                    .collect::<Vec<_>>()
            ),
        ),
    ]
}
