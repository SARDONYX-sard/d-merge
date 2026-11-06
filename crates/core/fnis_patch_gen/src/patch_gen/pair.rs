use std::borrow::Cow;

use fnis_list::{
    combinator::Trigger,
    patterns::pair_and_kill::{ActorRole, AnimObject, FNISPairedAndKillAnimation},
};
use json_patch::{ValueWithPriority, json_path, json_value};

use crate::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        JsonPatchPairs,
        classes::{
            ClipGenerator, StateInfo, StateMachine, new_anim_object, new_clip_trigger_array, new_variable_binding_set,
        },
        kill_move::{
            PairedAnimation, calculate_hash, make_event_state_info_patch, make_player_root_state_info_patch,
            new_event_property_array, new_npc_synchronized_clip_generator, new_player_synchronized_clip_generator,
            new_push_transitions_seq_patch,
        },
        new_push_events_seq_patch, new_seq_push,
    },
};

/// Into `meshes\actors\character\behaviors\0_master.xml`.(Search Keyword: `FNIS Begin Paired`)
pub(super) fn new_pair_patches<'a>(
    paired_and_kill_animation: FNISPairedAndKillAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let class_indexes: [&'a str; 22] = std::array::from_fn(|_| owned_data.next_class_name_attribute());

    let player_event = paired_and_kill_animation.anim_event;
    let class_index_0_id = calculate_hash(class_indexes[0]); // Must be 1 file unique
    let player_root_state_name = owned_data.keep(format!("Player_FNISpa{class_index_0_id}")); // Player_FNISpa$1/1$ NOTE: must be unique in 0_master.xml

    let npc_event = owned_data.keep(format!("pa_{player_event}"));
    let npc_root_state_name = owned_data.keep(format!("NPC_FNISpa{class_index_0_id}")); // NOTE: must be unique in 0_master.xml

    let anim_file =
        owned_data.keep(format!("Animations\\{}\\{}", owned_data.namespace, paired_and_kill_animation.anim_file)); // Animations\\$Fpa$
    let priority = owned_data.priority;
    let flags = paired_and_kill_animation.flag_set.flags;
    let duration = paired_and_kill_animation.flag_set.duration;

    let mut one_patches = vec![];
    let mut seq_patches = vec![];

    seq_patches.extend(new_push_events_seq_patch(
        vec![player_event.into(), npc_event.into()],
        owned_data.behavior_entry.master_string_data_index,
        owned_data.behavior_entry.master_behavior_graph_index,
        priority,
    ));

    seq_patches.push(new_push_transitions_seq_patch(
        "#0725", // By Search Keyword: `FNIS Begin Paired`
        "#0111",
        [player_event, npc_event],
        // The key point here is that `toStateId` should contain each other's IDs. (As revealed in `temporary_logs`)
        // player_event -> npc stateMachineInfo
        // npc_event -> player stateMachineInfo
        [npc_root_state_name, player_root_state_name], // toStateId
        priority,
    ));

    // Push and register the Root `hkbStateMachineStateInfo`(`MT_PairedAnimationsBehavior`) for both Player and NPC.
    seq_patches.push(new_seq_push(
        json_path!["#0724", "hkbStateMachine", "states"],
        priority,
        json_value!([class_indexes[0], class_indexes[11]]),
    ));

    // Associate the number of times an assigned index occurs with the name of the AnimObject at that time, and use this association to reference the eventID.
    // e.g. (#FNIS$1, 1)
    let mut active_indexes = Vec::new();
    let mut passive_indexes = Vec::new();
    for AnimObject { name, role } in &paired_and_kill_animation.anim_objects {
        let new_anim_object_index = owned_data.next_class_name_attribute();

        one_patches.push(new_anim_object(new_anim_object_index, name, priority));

        match role {
            ActorRole::Active => active_indexes.push(new_anim_object_index),
            ActorRole::Passive => passive_indexes.push(new_anim_object_index),
        }
    }

    let paired = PairedAnimation {
        animation: &paired_and_kill_animation,
        class_indexes: &class_indexes,
        class_index_0_id,
        player_event,
        npc_event,
        player_root_state_name,
        npc_root_state_name,
        anim_file,
        flags,
        duration,
        active_indexes: &active_indexes,
        passive_indexes: &passive_indexes,
        priority,
    };
    player_patches(&paired, &mut one_patches);
    npc_patches(&paired, &mut one_patches);

    (one_patches, seq_patches)
}

/// What the first actor plays: `$RI` to `$RI+10`.
fn player_patches<'a>(paired: &PairedAnimation<'a, '_>, one_patches: &mut JsonPatchPairs<'a>) {
    let PairedAnimation {
        animation,
        class_indexes,
        class_index_0_id,
        npc_event,
        player_root_state_name,
        anim_file,
        flags,
        duration,
        active_indexes,
        priority,
        ..
    } = *paired;

    // $RI
    one_patches.push(make_player_root_state_info_patch(
        class_indexes[0],
        class_indexes[1],
        flags,
        priority,
        player_root_state_name,
    ));

    // RI+1
    one_patches.push(
        StateMachine {
            index: class_indexes[1],
            variable_binding_set: class_indexes[2],
            name: format!("Player_FNISpa{class_index_0_id}_Behavior").into(), // Player_FNISpa$1/1$_Behavior
            states: vec![class_indexes[3]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // #$RI+2$  hkbVariableBindingSet
    one_patches.push(new_variable_binding_set(
        class_indexes[2],
        priority,
        json_value!([
            {
                "memberPath": "isActive",
                "variableIndex": 51, // bIsSynced
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // #$RI+3$  hkbStateMachineStateInfo
    one_patches.push(
        StateInfo {
            index: class_indexes[3],
            generator: class_indexes[4],
            name: format!("Player_FNISpa{class_index_0_id}_DisablePitch").into(),
            ..Default::default()
        }
        .into_patch(priority),
    );

    // #$RI+4$  hkbStateMachine
    one_patches.push(
        StateMachine {
            index: class_indexes[4],
            variable_binding_set: class_indexes[5],
            name: format!("Player_FNISpa{class_index_0_id}_DisablePitch_Behavior").into(),
            states: vec![class_indexes[6]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // #$RI+5$  hkbVariableBindingSet
    one_patches.push(new_variable_binding_set(
        class_indexes[5],
        priority,
        json_value!([
            {
                "memberPath": "isActive",
                "variableIndex": 58, // 2_FootScuffLeft
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // #$RI+6$  hkbStateMachineStateInfo
    one_patches.push(make_event_state_info_patch(
        class_indexes[6],
        flags,
        class_indexes[7],
        class_indexes[8],
        priority,
        format!("pa_FNISpa{class_index_0_id}"), // `pa_FNISpa_$1/1$`
    ));

    // #$RI+7$  hkbStateMachineEventPropertyArray
    //
    // "payload": "#$:AnimObj+&ao1$"
    one_patches.push(new_event_property_array(flags, active_indexes, class_indexes[7], priority));

    // #$RI+8$  BSSynchronizedClipGenerator
    one_patches.push(new_npc_synchronized_clip_generator(class_indexes[8], npc_event, class_indexes[9], priority));

    // #$RI+9$  hkbClipGenerator
    one_patches.push(
        ClipGenerator {
            index: class_indexes[9],
            name: format!("Paired_FNISpa{class_index_0_id}").into(),
            animation_name: anim_file.into(),
            triggers: class_indexes[10],
            mode: "MODE_SINGLE_PLAY",
        }
        .into_patch(priority),
    );

    // $RI+10
    //
    // The details are unclear, but 159 was not used—only 1070 was being used.
    //  156: NPCPairedStop
    //  159: PairEnd
    // 1070: NPCPairEnd
    one_patches.push(new_clip_trigger_array_patch(
        class_indexes[10],
        &[156, 1070],
        &animation.flag_set.triggers,
        duration,
        priority,
    ));
}

/// What the other actor plays: `$RI+11` to `$RI+21`.
fn npc_patches<'a>(paired: &PairedAnimation<'a, '_>, one_patches: &mut JsonPatchPairs<'a>) {
    let PairedAnimation {
        animation,
        class_indexes,
        class_index_0_id,
        player_event,
        npc_root_state_name,
        anim_file,
        flags,
        duration,
        passive_indexes,
        priority,
        ..
    } = *paired;

    // $RI+11
    // NOTE: kill move player root patch same as paired NPC.
    one_patches.push(make_player_root_state_info_patch(
        class_indexes[11],
        class_indexes[12],
        flags,
        priority,
        npc_root_state_name,
    ));

    // $RI+12
    one_patches.push(
        StateMachine {
            index: class_indexes[12],
            variable_binding_set: class_indexes[13],
            name: format!("NPC_FNISpa{class_index_0_id}_Behavior").into(),
            states: vec![class_indexes[14]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+13
    one_patches.push(new_variable_binding_set(
        class_indexes[13],
        priority,
        json_value!([
            {
                "memberPath": "isActive",
                "variableIndex": 51, // bIsSynced
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // $RI+14
    one_patches.push(
        StateInfo {
            index: class_indexes[14],
            generator: class_indexes[15],
            name: format!("NPC_FNISpa{class_index_0_id}_DisablePitch").into(),
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+15
    one_patches.push(
        StateMachine {
            index: class_indexes[15],
            variable_binding_set: class_indexes[16],
            name: format!("NPC_FNISpa{class_index_0_id}_DiablePitch_Behavior").into(), // not typo: `Diable`
            states: vec![class_indexes[17]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+16
    one_patches.push(new_variable_binding_set(
        class_indexes[16],
        priority,
        json_value!([
            {
                "memberPath": "isActive",
                "variableIndex": 58, // 2_FootScuffLeft
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // $RI+17 hkbStateMachineStateInfo
    one_patches.push(make_event_state_info_patch(
        class_indexes[17],
        flags,
        class_indexes[18],
        class_indexes[19],
        priority,
        format!("FNISpa{class_index_0_id}"), // FNISpa$1/1$
    ));

    // $RI+18
    //
    // "payload": "#$:AnimObj+&ao2$"
    one_patches.push(new_event_property_array(flags, passive_indexes, class_indexes[18], priority));

    // $RI+19
    one_patches.push(new_player_synchronized_clip_generator(
        class_indexes[19],
        player_event,
        class_indexes[20],
        priority,
    ));

    // $RI+20
    one_patches.push(
        ClipGenerator {
            index: class_indexes[20],
            name: format!("NPCPaired_FNISpa{class_index_0_id}").into(),
            animation_name: anim_file.into(),
            triggers: class_indexes[21],
            mode: "MODE_SINGLE_PLAY",
        }
        .into_patch(priority),
    );

    // $RI+21
    //
    // NOTE: Although it does not exist in the template,
    // 956(`2_pairedStop`) was included along with the duration in `temporary_logs`.(0_master_TEMPLATE.txt:5237 )
    one_patches.push(new_clip_trigger_array_patch(
        class_indexes[21],
        &[956],
        &animation.flag_set.triggers2,
        duration,
        priority,
    ));
}

fn new_clip_trigger_array_patch<'a>(
    class_index: &'a str,
    fixed_event_ids: &[u32],
    triggers: &[Trigger],
    duration: f32,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut trigger_values = Vec::with_capacity(fixed_event_ids.len() + triggers.len());

    trigger_values.extend(triggers.iter().map(|Trigger { event, time }| {
        json_value!({
            "localTime": *time,
            "event": {
                "id": format!("$eventID[{event}]$"),
                "payload": "#0000"
            },
            "relativeToEndOfClip": false,
            "acyclic": false,
            "isAnnotation": false
        })
    }));

    trigger_values.extend(fixed_event_ids.iter().map(|&id| {
        json_value!({
            "localTime": duration,
            "event": {
                "id": id,
                "payload": "#0000"
            },
            "relativeToEndOfClip": false,
            "acyclic": false,
            "isAnnotation": false
        })
    }));

    new_clip_trigger_array(class_index, priority, json_value!(trigger_values))
}
