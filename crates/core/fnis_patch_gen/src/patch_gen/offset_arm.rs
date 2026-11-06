//! NOTE: To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt"
use std::borrow::Cow;

use fnis_list::combinator::{flags::FNISAnimFlags, fnis_animation::FNISAnimation};
use json_patch::{ValueWithPriority, json_path, json_value};

use crate::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        JsonPatchPairs,
        classes::{ClipGenerator, StateInfo, new_anim_object, new_event_array},
        global::mt_behavior::{self, FNIS_AA_MT_AUTO_GEN_5235},
        kill_move::{calculate_hash, new_push_transitions_seq_patch},
        new_push_events_seq_patch, new_seq_push,
    },
};

/// # Target Template
/// `meshes\actors\character\behaviors\mt_behavior.xml`.
pub(super) fn new_offset_arm_patches<'a>(
    animation: &FNISAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [&'a str; 5] = std::array::from_fn(|_| owned_data.next_class_name_attribute());
    let namespace = &owned_data.namespace;
    let priority = owned_data.priority;
    let flags = animation.flag_set.flags;

    // NOTE: Assign a unique ID for each sentence in `ofa`(OffsetArm) to the event + Right & Left stateId.
    let class_index_0_id = calculate_hash(class_indexes[0]); // Must be 1 file unique

    let class_index_1_id = calculate_hash(class_indexes[1]); // Must be 1 file unique

    let right_offset_event = format!("FNIS_RightOffsetCG{class_index_0_id}");
    let left_offset_event = format!("FNIS_LeftOffsetCG{class_index_1_id}");
    let anim_file = owned_data.keep(format!("Animations\\{namespace}\\{}", animation.anim_file)); // Animations\\$Foa$

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    let right_offset_state_name = format!("FNIS_RightOffset{class_index_0_id}");
    let left_offset_state_name = format!("FNIS_LeftOffset{class_index_1_id}");

    seq_patches.extend(new_push_events_seq_patch(vec![animation.anim_event.into()], "#0083", "#0085", priority));

    seq_patches.push(new_push_transitions_seq_patch(
        "#4038",
        FNIS_AA_MT_AUTO_GEN_5235,
        [animation.anim_event],
        [class_indexes[0]],
        priority,
    ));

    // Push and register the Root `hkbStateMachineStateInfo` for both Right & Left offset arms.
    seq_patches.push(new_seq_push(
        json_path!["#5138", "hkbStateMachine", "states"],
        priority,
        json_value!([class_indexes[0], class_indexes[2]]),
    ));

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
    one_patches.push(
        StateInfo {
            index: class_indexes[0],
            transitions: "#5111",
            generator: class_indexes[1],
            name: right_offset_state_name.into(),
            state_id: class_index_0_id, // $99/1$
            ..Default::default()
        }
        .into_patch(priority),
    );

    // #$RI+1$  hkbClipGenerator
    one_patches.push({
        // $-a|MODE_SINGLE_PLAY|MODE_LOOPING$
        let mode = if flags.contains(FNISAnimFlags::Acyclic) { "MODE_SINGLE_PLAY" } else { "MODE_LOOPING" };

        ClipGenerator {
            index: class_indexes[1],
            name: right_offset_event.into(),
            animation_name: anim_file.into(),
            triggers: "#0000",
            mode,
        }
        .into_patch(priority)
    });

    // #$RI+2$  hkbStateMachineStateInfo
    one_patches.push({
        // $-o|#%RI+4%|h|null|#5219$
        let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects) {
            class_indexes[4]
        } else if flags.contains(FNISAnimFlags::HeadTracking) {
            "#0000"
        } else {
            mt_behavior::FNIS_AA_MT_AUTO_GEN_5219
        };
        // $h|null|#5218$
        let exit_notify_events =
            if flags.contains(FNISAnimFlags::HeadTracking) { "#0000" } else { mt_behavior::FNIS_AA_MT_AUTO_GEN_5218 };
        // $-o|#5147|#5152$
        let transition = if flags.contains(FNISAnimFlags::AnimObjects) { "#5147" } else { "#5152" };

        StateInfo {
            index: class_indexes[2],
            enter_notify_events,
            exit_notify_events,
            transitions: transition,
            generator: class_indexes[3],
            name: left_offset_state_name.into(),
            state_id: class_index_0_id,
        }
        .into_patch(priority)
    });

    // TODO: refactor $RI+1 & $RI+3 are same pattern
    //
    // #$RI+3$  hkbClipGenerator
    one_patches.push({
        // $-a|MODE_SINGLE_PLAY|MODE_LOOPING$
        let mode = if flags.contains(FNISAnimFlags::Acyclic) { "MODE_SINGLE_PLAY" } else { "MODE_LOOPING" };

        ClipGenerator {
            index: class_indexes[3],
            name: left_offset_event.into(),
            animation_name: anim_file.into(), // Animations\\Fofa$
            triggers: "#0000",
            mode,
        }
        .into_patch(priority)
    });

    // #$RI+4$  hkbStateMachineEventPropertyArray
    one_patches.push(new_event_property_array(flags, anim_object_indexes.first().copied(), class_indexes[4], priority));

    (one_patches, seq_patches)
}

/// # Note
/// `id` is `hkbBehaviorGraphStringData.eventNames` index
#[must_use]
fn new_event_property_array<'a>(
    flags: FNISAnimFlags,
    anim_object_index: Option<&'a str>,
    class_index: &'a str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = if flags.contains(FNISAnimFlags::HeadTracking) {
        vec![]
    } else {
        vec![json_value!({
            "id": 20, // HeadTrackingOff
            "payload": "#0000"
        })]
    };

    if let Some(anim_object_index) = anim_object_index {
        // 393: AnimObjLoad
        // 394: AnimObjDraw
        events.extend([393, 394].into_iter().map(|id| {
            json_value!({
                    "id": id,
                    "payload": anim_object_index
            })
        }));
    }

    new_event_array(class_index, priority, json_value!(events))
}
