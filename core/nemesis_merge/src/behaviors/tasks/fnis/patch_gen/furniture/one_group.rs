//! To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
use std::borrow::Cow;

use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::FNISAnimFlags;
use crate::behaviors::tasks::fnis::list_parser::patterns::furniture::FurnitureAnimation;
use crate::behaviors::tasks::fnis::patch_gen::furniture::one_anim::{
    new_furniture_one_anim_patches, FurniturePhase,
};
use crate::behaviors::tasks::fnis::patch_gen::global::mt_behavior::{
    FNIS_AA_MT_AUTO_GEN_5220, FNIS_AA_MT_AUTO_GEN_5221, FNIS_BA_BLEND_TRANSITION_5231,
    FNIS_BA_BLEND_TRANSITION_5232, FNIS_FU_MT_5216, FNIS_GLOBAL_FU_MT_STATE_ID,
};
use crate::behaviors::tasks::fnis::patch_gen::{kill_move::calculate_hash, JsonPatchPairs};

/// This patch treats a single piece of furniture as consisting of at least 4 animations.
///
/// # Note
/// Not yet tested.
///
/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub fn new_furniture_one_group_patches<'a>(
    furniture: &FurnitureAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>, String) {
    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // Furniture seq iter
    let total = furniture.animations.len();

    // par animation class indexes
    let all_class_indexes: Vec<[String; 9]> = furniture
        .animations
        .par_iter()
        .map(|_| core::array::from_fn(|_| owned_data.next_class_name_attribute()))
        .collect();

    // Why need this?
    // Analysis of the FNIS output reveals that the stateId for each $RI+3$ index in the Furniture sequence must be set to the sequence's final stateId.
    let end_anim_state_id = match all_class_indexes.last() {
        Some(lasts) => calculate_hash(&lasts[0]),
        None => 0, // Since the list parser already validates that Furniture must consist of at least three items, this is actually unreachable.
    };

    for (i, (animation, class_indexes)) in furniture
        .animations
        .iter()
        .zip(&all_class_indexes)
        .enumerate()
    {
        let phase = match i {
            0 => FurniturePhase::Start,
            n if n == total - 1 => FurniturePhase::End(total - 1),
            n => FurniturePhase::Middle(n),
        };

        let next_event_name = furniture.animations.get(i + 1).map(|next| next.anim_event);

        let (one, seq) = new_furniture_one_anim_patches(
            animation,
            owned_data,
            phase,
            class_indexes,
            end_anim_state_id,
            next_event_name,
        );

        one_patches.par_extend(one);
        seq_patches.par_extend(seq);
    }

    ////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    // One Furniture seq root

    let class_indexes: [String; 6] =
        std::array::from_fn(|_| owned_data.next_class_name_attribute());
    let priority = owned_data.priority;

    // Safety(no-panic): Since the list parser checks for length, the first animation must always exist.
    let first_animation = &furniture.animations[0];
    let first_animation_event_id = format!("$eventID[{}]$", first_animation.anim_event);
    let first_animation_flags = first_animation.flag_set.flags;
    let first_animation_vars = first_animation.flag_set.anim_vars.as_slice();

    // Push the first animation for furniture seq
    seq_patches.push({
        let first_animation_state_id = match all_class_indexes.first() {
            Some(first) => calculate_hash(&first[0]),
            None => 0, // Since the list parser already validates that Furniture must consist of at least three items, this is actually unreachable.
        };

        (
            json_patch::json_path!["#0089", "hkbStateMachineTransitionInfoArray", "transitions"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: json_typed!(borrowed, [{
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
                        "transition": FNIS_FU_MT_5216,
                        "condition": "#0000",
                        // eventId is Nemesis variable, derived from `events`
                        "eventId": first_animation_event_id,
                        // toStateId must match root_event (NOT the event)
                        "toStateId": FNIS_GLOBAL_FU_MT_STATE_ID,
                        "fromNestedStateId": 0,
                        "toNestedStateId": first_animation_state_id,
                        "priority": 0,
                        "flags": "FLAG_TO_NESTED_STATE_ID_IS_VALID|FLAG_IS_LOCAL_WILDCARD|FLAG_DISABLE_CONDITION"
                    }]),
                },
                priority,
            },
        )
    });

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let class_index_to_anim_object_map = dashmap::DashMap::new();
    one_patches.par_extend(first_animation.anim_objects.par_iter().enumerate().map(
        |(index, name)| {
            let new_anim_object_index = owned_data.next_class_name_attribute();
            class_index_to_anim_object_map.insert(index, new_anim_object_index.clone());
            let one_anim_obj = (
                vec![
                    Cow::Owned(new_anim_object_index.clone()),
                    Cow::Borrowed("hkbStringEventPayload"),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Pure { op: Op::Add },
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": new_anim_object_index,
                            "data": name, // StringPtr
                        }),
                    },
                    priority,
                },
            );
            one_anim_obj
        },
    ));

    // Root
    // NOTE: Assign a unique ID for each furniture sequence. Must be 1 file unique.
    let class_index_0_id = calculate_hash(&class_indexes[0]);
    let furniture_one_seq_state_name = format!("FNIS_Furniture{class_index_0_id}_State"); // FNIS_Furniture$1/1$_State

    // $RI
    one_patches.push({
        // $-ac1|#5220|null$
        let enter_notify_events =
            if first_animation_flags.contains(FNISAnimFlags::AnimatedCameraSet) {
                FNIS_AA_MT_AUTO_GEN_5220
            } else {
                "#0000"
            };
        // $-ac0|#5221|null$
        let exit_notify_events =
            if first_animation_flags.contains(FNISAnimFlags::AnimatedCameraReset) {
                FNIS_AA_MT_AUTO_GEN_5221
            } else {
                "#0000"
            };

        // #$-AV|%RI+1%|%RI+4%$
        let generator = if first_animation_vars.is_empty() {
            &class_indexes[4]
        } else {
            &class_indexes[1]
        };

        (
            vec![
                Cow::Owned(class_indexes[0].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[0],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": "#0000",
                        "generator": generator,
                        "name": furniture_one_seq_state_name,
                        "stateId": class_index_0_id, // $1/1$
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });

    // <!-- $-AV$ -->
    if !first_animation_vars.is_empty() {
        // RI+2
        one_patches.push((
            vec![
                Cow::Owned(class_indexes[1].clone()),
                Cow::Borrowed("hkbModifierGenerator"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[1],
                        "variableBindingSet": "#0000", // null
                        "userData": 1,
                        "name": "IdleMesmerizes_MG",
                        "modifier": class_indexes[2],
                        "generator": class_indexes[4],
                    }),
                },
                priority,
            },
        ));

        // #RI+2
        one_patches.push({
            let inverse_flags: [bool; 5] =
                core::array::from_fn(|i| first_animation_vars.get(i).is_some_and(|v| v.inverse));

            (
                vec![
                    Cow::Owned(class_indexes[2].clone()),
                    Cow::Borrowed("BSIsActiveModifier"),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Pure { op: Op::Add },
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": class_indexes[2],
                            "variableBindingSet": &class_indexes[3],
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
                    },
                    priority,
                },
            )
        });

        one_patches.push({
            let bindings: Vec<_> = first_animation_vars
                .par_iter()
                .take(5) // FIXME?: Is this correct?: The `BSIsActiveModifier` field up to 0..=4
                .enumerate()
                .map(|(i, var)| {
                    simd_json::json_typed!(borrowed, {
                        "memberPath": format!("bIsActive{i}"),
                        "variableIndex": format!("$variableID{}$", var.name),  // $&AVI$
                        "bitIndex": -1,
                        "bindingType": "BINDING_TYPE_VARIABLE"
                    })
                })
                .collect();

            (
                vec![
                    Cow::Owned(class_indexes[3].clone()),
                    Cow::Borrowed("hkbVariableBindingSet"),
                ],
                ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Pure { op: Op::Add },
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": class_indexes[3],
                            "bindings": bindings,
                            "indexOfBindingToEnable": -1
                        }),
                    },
                    priority,
                },
            )
        });
    }

    // furniture seq hkbStateMachineStateInfo indexes
    let all_state_infos_indexes: Vec<&str> = all_class_indexes
        .iter()
        .map(|indexes| indexes[0].as_str())
        .collect();

    // RI+4
    one_patches.push({
        (
            vec![
                Cow::Owned(class_indexes[4].clone()),
                Cow::Borrowed("hkbStateMachine"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": class_indexes[4],
                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": format!("FNIS_Furniture{class_index_0_id}_Behavior"),
                        "eventToSendWhenStateOrTransitionChanges": {
                            "id": -1,
                            "payload": "#0000"
                        },
                        "startStateChooser": "#0000",
                        "startStateId": 1,
                        "returnToPreviousStateEventId": -1,
                        "randomTransitionEventId": -1,
                        "transitionToNextHigherStateEventId": -1,
                        "transitionToNextLowerStateEventId": -1,
                        "syncVariableIndex": -1,
                        "wrapAroundStateId": false,
                        "maxSimultaneousTransitions": 32,
                        "startStateMode": "START_STATE_MODE_DEFAULT",
                        "selfTransitionMode": "SELF_TRANSITION_MODE_NO_TRANSITION",
                        "states": all_state_infos_indexes, // #$!fu$ -> all furniture's hkbStateMachineStateInfo indexes
                        "wildcardTransitions": class_indexes[5]
                    }),
                },
                priority,
            },
        )
    });

    // #$RI+5$ hkbStateMachineTransitionInfoArray
    one_patches.push({
        let transitions: Vec<_> = all_state_infos_indexes.iter().enumerate().map(|(index, class_indexes)| {
            let blend_class_index = if index == 0 {
                FNIS_BA_BLEND_TRANSITION_5231
            } else {
                FNIS_BA_BLEND_TRANSITION_5232
            };

            json_typed!(borrowed, {
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
                "transition": blend_class_index, // #$:BlendTransition+&bl$
                "condition": "#0000",
                "eventId": first_animation_event_id, // $AE1fu+%fu$
                "toStateId": calculate_hash(class_indexes), // $1/1$
                "fromNestedStateId": 0,
                "toNestedStateId": 0,
                "priority": 0,
                "flags": "FLAG_IS_LOCAL_WILDCARD|FLAG_IS_GLOBAL_WILDCARD|FLAG_DISABLE_CONDITION",
            })
        }).collect();

        (
            vec![
                Cow::Owned(class_indexes[5].clone()),
                Cow::Borrowed("hkbStateMachineTransitionInfoArray"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[5],
                        "transitions": transitions,
                    }),
                },
                priority,
            },
        )
    });

    (one_patches, seq_patches, class_indexes[0].clone())
}
