//! NOTE: To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
use std::borrow::Cow;

use json_patch::{json_path, JsonPatch, Op, OpRangeKind, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::combinator::flags::FNISAnimFlags;
use crate::behaviors::tasks::fnis::list_parser::combinator::fnis_animation::FNISAnimation;
use crate::behaviors::tasks::fnis::patch_gen::global::mt_behavior;
use crate::behaviors::tasks::fnis::patch_gen::kill_move::new_push_transitions_seq_patch;
use crate::behaviors::tasks::fnis::patch_gen::{
    kill_move::calculate_hash, JsonPatchPairs, PUSH_OP,
};

/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub fn new_offset_arm_patches<'a>(
    animation: &FNISAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [String; 5] =
        std::array::from_fn(|_| owned_data.next_class_name_attribute());
    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let flags = animation.flag_set.flags;
    let right_offset_event = format!("FNIS_RightOffsetCG{priority}");
    let left_offset_event = format!("FNIS_LeftOffsetCG{priority}");
    let anim_file = format!("Animations\\{namespace}\\{}", animation.anim_file); // Animations\\$Foa$

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    let right_offset_state_name = format!("FNIS_RightOffset{priority}");
    let left_offset_state_name = format!("FNIS_LeftOffset{priority}");
    seq_patches.push(new_push_transitions_seq_patch(
        "#4038",
        [right_offset_event.as_str(), left_offset_event.as_str()],
        [&right_offset_state_name, &left_offset_state_name],
        priority,
    ));

    // Push and register the Root `hkbStateMachineStateInfo` for both Right & Left offset arms.
    seq_patches.push((
        json_path!["#5138", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, [class_indexes[0], class_indexes[2]]),
            },
            priority,
        },
    ));

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let class_index_to_anim_object_map = dashmap::DashMap::new();
    one_patches.par_extend(
        animation
            .anim_objects
            .par_iter()
            .enumerate()
            .map(|(index, name)| {
                let new_anim_object_index = owned_data.next_class_name_attribute();
                class_index_to_anim_object_map.insert(index, new_anim_object_index.clone());
                let one_anim_obj = (
                    vec![
                        Cow::Owned(new_anim_object_index.clone()),
                        Cow::Borrowed("hkbStringEventPayload"),
                    ],
                    ValueWithPriority {
                        patch: JsonPatch {
                            op: OpRangeKind::Pure(Op::Add),
                            value: simd_json::json_typed!(borrowed, {
                                "__ptr": new_anim_object_index,
                                "data": name, // StringPtr
                            }),
                        },
                        priority,
                    },
                );
                one_anim_obj
            }),
    );

    // $RI
    one_patches.push((
        vec![
            Cow::Owned(class_indexes[0].clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_indexes[0],
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": "#5111",
                    "generator": &class_indexes[1],
                    "name": right_offset_state_name,
                    "stateId": calculate_hash(&right_offset_state_name), // $99/1$
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    // #$RI+1$  hkbClipGenerator
    one_patches.push({
        // $-a|MODE_SINGLE_PLAY|MODE_LOOPING$
        let mode = if flags.contains(FNISAnimFlags::Acyclic) {
            "MODE_SINGLE_PLAY"
        } else {
            "MODE_LOOPING"
        };

        (
            vec![
                Cow::Owned(class_indexes[1].clone()),
                Cow::Borrowed("hkbClipGenerator"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[1],
                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": format!("FNIS_RightOffsetCG{priority}"),
                        "animationName": anim_file,
                        "triggers": "#0000",
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
                },
                priority,
            },
        )
    });

    // #$RI+2$  hkbStateMachineStateInfo
    one_patches.push({
        // $-o|#%RI+4%|h|null|#5219$
        let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects) {
            class_indexes[4].as_str()
        } else if flags.contains(FNISAnimFlags::HeadTracking) {
            "#0000"
        } else {
            mt_behavior::FNIS_AA_MT_AUTO_GEN_5219
        };
        // $h|null|#5218$
        let exit_notify_events = if flags.contains(FNISAnimFlags::HeadTracking) {
            "#0000"
        } else {
            mt_behavior::FNIS_AA_MT_AUTO_GEN_5218
        };
        // $-o|#5147|#5152$
        let transition = if flags.contains(FNISAnimFlags::AnimObjects) {
            "#5147"
        } else {
            "#5152"
        };

        (
            vec![
                Cow::Owned(class_indexes[2].clone()),
                Cow::Borrowed("hkbStateMachineStateInfo"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[2],
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": enter_notify_events,
                        "exitNotifyEvents": exit_notify_events,
                        "transitions": transition,
                        "generator": &class_indexes[3],
                        "name": left_offset_event,
                        "stateId": 0,
                        "probability": 1.0,
                        "enable": true
                    }),
                },
                priority,
            },
        )
    });

    // TODO: refactor $RI+1 & $RI+3 are same pattern
    //
    // #$RI+3$  hkbClipGenerator
    one_patches.push({
        // $-a|MODE_SINGLE_PLAY|MODE_LOOPING$
        let mode = if flags.contains(FNISAnimFlags::Acyclic) {
            "MODE_SINGLE_PLAY"
        } else {
            "MODE_LOOPING"
        };

        (
            vec![
                Cow::Owned(class_indexes[3].clone()),
                Cow::Borrowed("hkbClipGenerator"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    op: OpRangeKind::Pure(Op::Add),
                    value: json_typed!(borrowed, {
                        "__ptr": class_indexes[3],
                        "variableBindingSet": "#0000",
                        "userData": 0,
                        "name": format!("FNIS_LeftOffsetCG{priority}"),
                        "animationName": anim_file, // Animations\\Fofa$
                        "triggers": "#0000",
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
                },
                priority,
            },
        )
    });

    // #$RI+4$  hkbStateMachineEventPropertyArray
    one_patches.push({
        let anim_obj_class_index = class_index_to_anim_object_map.get(&0).map(|v| v.clone());
        new_event_property_array(flags, anim_obj_class_index, &class_indexes[4], priority)
    });

    (one_patches, seq_patches)
}

/// # Note
/// `id` is `hkbBehaviorGraphStringData.eventNames` index
#[must_use]
fn new_event_property_array<'a>(
    flags: FNISAnimFlags,
    anim_object_index: Option<String>,
    class_index: &str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = if flags.contains(FNISAnimFlags::HeadTracking) {
        vec![]
    } else {
        vec![json_typed!(borrowed, {
            "id": 20, // HeadTrackingOff
            "payload": "#0000"
        })]
    };

    if let Some(anim_object_index) = anim_object_index {
        // 393: AnimObjLoad
        // 394: AnimObjDraw
        events.extend([393, 394].iter().map(|id| {
            json_typed!(borrowed, {
                    "id": id,
                    "payload": anim_object_index
            })
        }));
    }

    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbStateMachineEventPropertyArray"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": class_index,
                    "events": events
                }),
            },
            priority,
        },
    )
}
