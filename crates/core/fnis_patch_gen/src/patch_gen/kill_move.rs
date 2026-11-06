//! NOTE: To learn the additional method, "FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\0_master_TEMPLATE.txt"
use std::borrow::Cow;

use fnis_list::{
    combinator::{Trigger, flags::FNISAnimFlags},
    patterns::pair_and_kill::{ActorRole, AnimObject, FNISPairedAndKillAnimation},
};
use json_patch::{Action, JsonPatch, ValueWithPriority, json_path, json_value};
use rayon::prelude::*;

use crate::{
    collect::owned::OwnedFnisInjection,
    patch_gen::{
        JsonPatchPairs,
        classes::{
            ClipGenerator, ModifierGenerator, StateInfo, StateMachine, new_anim_object, new_clip_trigger_array,
            new_event_array, new_variable_binding_set,
        },
        global::_0_master::{
            FNIS_AA_GLOBAL_AUTO_GEN_2526, FNIS_AA_GLOBAL_AUTO_GEN_2527, FNIS_AA_GLOBAL_AUTO_GEN_2528,
            FNIS_AA_GLOBAL_AUTO_GEN_2529, FNIS_AA_GLOBAL_AUTO_GEN_2530, FNIS_AA_GLOBAL_AUTO_GEN_2532,
            FNIS_AA_GLOBAL_AUTO_GEN_2533, FNIS_AA_GLOBAL_AUTO_GEN_2534,
        },
        new_class_patch, new_push_events_seq_patch, new_seq_push,
    },
};

/// What the patches of a paired animation (`pa`, `km`) of two actors have in common.
#[derive(Clone, Copy)]
pub(super) struct PairedAnimation<'a, 'b> {
    pub animation: &'b FNISPairedAndKillAnimation<'a>,
    /// The new classes, in the order the `$RI+n` comments count them.
    pub class_indexes: &'b [&'a str],
    pub class_index_0_id: i32,
    pub player_event: &'a str,
    /// `pa_<player_event>`
    pub npc_event: &'a str,
    pub player_root_state_name: &'a str,
    pub npc_root_state_name: &'a str,
    /// e.g. `Animations\FNISZoo\back_grab.hkx`
    pub anim_file: &'a str,
    pub flags: FNISAnimFlags,
    pub duration: f32,
    /// The animation objects of each actor. See `AnimObject`.
    pub active_indexes: &'b [&'a str],
    pub passive_indexes: &'b [&'a str],
    pub priority: usize,
}

/// Into `meshes\actors\character\behaviors\0_master.xml`.
pub(super) fn new_kill_patches<'a>(
    paired_and_kill_animation: FNISPairedAndKillAnimation<'a>,
    owned_data: &'a OwnedFnisInjection,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    // new C++ Havok class XML name attributes
    let class_indexes: [&'a str; 26] = core::array::from_fn(|_| owned_data.next_class_name_attribute());

    let player_event = paired_and_kill_animation.anim_event;
    let class_index_0_id = calculate_hash(class_indexes[0]); // Must be 1 file unique
    let player_root_state_name = owned_data.keep(format!("Player_FNISkm{class_index_0_id}")); // NOTE: must be unique in 0_master.xml

    let npc_event = owned_data.keep(format!("pa_{player_event}")); // NOTE: Since items in eventNames are inherently unique, this approach is acceptable.
    let class_index_12_id = calculate_hash(class_indexes[12]); // Must be 1 file unique
    let npc_root_state_name = owned_data.keep(format!("NPC_FNISkm{class_index_12_id}")); // NOTE: must be unique in 0_master.xml

    let anim_file =
        owned_data.keep(format!("Animations\\{}\\{}", owned_data.namespace, paired_and_kill_animation.anim_file)); // Animations\\$Fkm$
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
        "#0789",
        "#0111",
        [player_event, npc_event],
        [player_root_state_name, npc_root_state_name],
        priority,
    ));
    // Push and register the Root `hkbStateMachineStateInfo` for both Player and NPC.
    seq_patches.push(new_seq_push(
        json_path!["#0788", "hkbStateMachine", "states"],
        priority,
        json_value!([class_indexes[0], class_indexes[12]]),
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

/// What the killer plays: `$RI` to `$RI+11`.
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

    // $RI+1
    one_patches.push(
        ModifierGenerator {
            index: class_indexes[1],
            name: format!("Player_FNISkm{class_index_0_id}_ModGen").into(), // StringPtr
            modifier: class_indexes[14],
            generator: class_indexes[2],
        }
        .into_patch(priority),
    );

    // $RI+2
    one_patches.push(
        StateMachine {
            index: class_indexes[2],
            variable_binding_set: class_indexes[3],
            name: format!("Player_FNISkm{class_index_0_id}$_Behavior").into(), // FIXME? $1/1$
            states: vec![class_indexes[4]],
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
                "variableIndex": 51, // bIsSynced
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // $RI+4
    one_patches.push(
        StateInfo {
            index: class_indexes[4],
            generator: class_indexes[5],
            name: format!("Player_FNISkm{class_index_0_id}_DisablePitch").into(),
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+5
    one_patches.push(
        StateMachine {
            index: class_indexes[5],
            variable_binding_set: class_indexes[6],
            name: format!("Player_FNISkm{class_index_0_id}_DisablePitch_Behavior").into(),
            states: vec![class_indexes[7]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+6
    one_patches.push(new_variable_binding_set(
        class_indexes[6],
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

    // $RI+7
    one_patches.push(make_event_state_info_patch(
        class_indexes[7],
        flags,
        class_indexes[8],
        class_indexes[9],
        priority,
        npc_event, // pa_$Ekm$
    ));

    // $RI+8
    one_patches.push({
        // "payload": "#$:AnimObj+&ao1$"
        new_event_property_array(flags, active_indexes, class_indexes[8], priority)
    });

    // $RI+9
    one_patches.push(new_npc_synchronized_clip_generator(class_indexes[9], npc_event, class_indexes[10], priority));

    // $RI+10
    one_patches.push(
        ClipGenerator {
            index: class_indexes[10],
            name: format!("Paired_FNISkm{class_index_0_id}").into(), // FIXME?: priority <- $1/1$
            animation_name: anim_file.into(),                        // Animations\\$Fkm$
            triggers: class_indexes[11],
            mode: "MODE_SINGLE_PLAY",
        }
        .into_patch(priority),
    );

    // $RI+11
    one_patches.push({
        // "id": 615, // NPCKillMoveStart
        let mut triggers = new_values_from_triggers(615, &animation.flag_set.triggers);

        //  156: NPCPairedStop
        //  616: NPCKillMoveEnd
        // 1070: NPCPairEnd (159: PairEnd)
        triggers.par_extend([156, 616, 1070].par_iter().map(|&id| {
            json_value!({
                "localTime": duration, // $-D$
                "event": {
                    "id": id,
                    "payload": "#0000"
                },
                "relativeToEndOfClip": false,
                "acyclic": false,
                "isAnnotation": false
            })
        }));

        new_clip_trigger_array(class_indexes[11], priority, json_value!(triggers))
    });
}

/// What the victim plays: `$RI+12` to `$RI+25`.
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

    // $RI+12
    one_patches.push(make_npc_root_state_info_patch(
        class_indexes[12],
        class_indexes[13],
        flags,
        priority,
        npc_root_state_name,
    ));

    // $RI+13
    one_patches.push(
        ModifierGenerator {
            index: class_indexes[13],
            name: format!("NPC_FNISkm{class_index_0_id}_ModGen").into(),
            modifier: class_indexes[14],
            generator: class_indexes[16],
        }
        .into_patch(priority),
    );

    // $RI+14
    one_patches.push(new_class_patch(
        class_indexes[14],
        "BSIsActiveModifier",
        priority,
        json_value!({
            "__ptr": class_indexes[14],
            "variableBindingSet": class_indexes[15],
            "userData": 2,
            "name": format!("FNISkm{class_index_0_id}_ActiveModifier"),
            "enable": true,
            "bIsActive0": false,
            "bInvertActive0": false,
            "bIsActive1": false,
            "bInvertActive1": false,
            "bIsActive2": false,
            "bInvertActive2": false,
            "bIsActive3": false,
            "bInvertActive3": false,
            "bIsActive4": false,
            "bInvertActive4": false
        }),
    ));

    // $RI+15
    one_patches.push(new_variable_binding_set(
        class_indexes[15],
        priority,
        json_value!([
            {
                "memberPath": "bIsActive0",
                "variableIndex": 213,
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // $RI+16
    one_patches.push(
        StateMachine {
            index: class_indexes[16],
            variable_binding_set: class_indexes[17],
            name: format!("NPC_FNISkm{class_index_0_id}_Behavior").into(),
            states: vec![class_indexes[18]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+17
    one_patches.push(new_variable_binding_set(
        class_indexes[17],
        priority,
        json_value!([
            {
                "memberPath": "bIsActive0",
                "variableIndex": 51, // bIsSynced
                "bitIndex": -1,
                "bindingType": "BINDING_TYPE_VARIABLE"
            }
        ]),
    ));

    // $RI+18
    one_patches.push(
        StateInfo {
            index: class_indexes[18],
            generator: class_indexes[19],
            name: format!("NPC_FNISkm{class_index_0_id}_DisablePitch").into(),
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+19
    one_patches.push(
        StateMachine {
            index: class_indexes[19],
            variable_binding_set: class_indexes[20],
            name: format!("NPC_FNISkm{class_index_0_id}_DisablePitch_Behavior").into(),
            states: vec![class_indexes[21]],
            ..Default::default()
        }
        .into_patch(priority),
    );

    // $RI+20
    one_patches.push(new_variable_binding_set(
        class_indexes[20],
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

    // $RI+21
    one_patches.push(make_event_state_info_patch(
        class_indexes[21],
        flags,
        class_indexes[22],
        class_indexes[23],
        priority,
        player_event,
    ));

    // $RI+22
    one_patches.push({
        // "payload": "#$:AnimObj+&ao2$"
        new_event_property_array(flags, passive_indexes, class_indexes[22], priority)
    });

    // $RI+23
    one_patches.push(new_player_synchronized_clip_generator(
        class_indexes[23],
        player_event,
        class_indexes[24],
        priority,
    ));

    // $RI+24
    one_patches.push(
        ClipGenerator {
            index: class_indexes[24],
            name: format!("NPCPaired_FNISkm{class_index_0_id}").into(),
            animation_name: anim_file.into(), // Animations\\$Fkm$
            triggers: class_indexes[25],
            mode: "MODE_SINGLE_PLAY",
        }
        .into_patch(priority),
    );

    // $RI+25
    one_patches.push({
        // Syntaxes starting with `T2`, such as `T2_KillActor/5.867`, are processed here.
        let mut triggers = new_values_from_triggers(614, &animation.flag_set.triggers2);

        // NOTE: The insertion of the following triggers is not documented even in 0_master.TEMPLATE.txt.
        //       This was discovered by reading the differences from `temporary_logs/0_master.xml`.
        // - event index list
        //    622: 2_KillMoveEnd
        //    167: 2_KillActor
        //   1120: 2_PairEnd
        triggers.par_extend([622, 167, 1120].par_iter().map(|&id| {
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

        new_clip_trigger_array(class_indexes[25], priority, json_value!(triggers))
    });
}

/// New `hkbStateMachineStateInfo` patch.
/// - `state_name`:  e.g. `Player_FNIS_km{class_index_0_id}`, `Player_FNISpa$1/1$`
///
/// # Note
/// - `enter_notify_events`: #2530 or null
/// - `exit_notify_events`: #2532 or null
#[must_use]
pub(super) fn make_player_root_state_info_patch<'a>(
    class_index: &'a str,
    generator_index: &'a str,
    flags: FNISAnimFlags,
    priority: usize,
    state_name: &'a str,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    // NOTE: Why contains `ac`(`FNISAnimFlags::AnimatedCamera`)?
    // Based on the template, it doesn't seem to be enabled when -ac is used,
    // but in the actual `temporary_logs`, it wasn't null when -ac was enabled.
    //
    // Therefore, either the `AnimatedCamera` flag or the corresponding
    // `AnimatedCameraSet`/`AnimatedCameraReset` flag enables these events.

    // -ac1|#2530|null
    let enter_notify_events = match flags.intersects(FNISAnimFlags::AnimatedCamera | FNISAnimFlags::AnimatedCameraSet) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2530,
        false => "#0000",
    };
    // -ac0|#2532|null
    let exit_notify_events = match flags.intersects(FNISAnimFlags::AnimatedCamera | FNISAnimFlags::AnimatedCameraReset)
    {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2532,
        false => "#0000",
    };

    StateInfo {
        index: class_index,
        enter_notify_events,
        exit_notify_events,
        generator: generator_index,
        state_id: calculate_hash(state_name),
        name: state_name.into(), // pa: `$171/2$` km: `$148/2$`
        ..Default::default()
    }
    .into_patch(priority)
}

/// New `hkbStateMachineStateInfo` patch
///
/// - `state_name`:  e.g. `pa_{event_name}`, `pa_$Ekm` -> `pa_back`
///
/// # Note
/// - `enter_notify_events`: index or null or #2526
/// - `exit_notify_events`: `$-h,o|#2528|h|null|o|#2529|#2527$`
#[must_use]
pub(super) fn make_event_state_info_patch<'a>(
    class_index: &'a str,
    flags: FNISAnimFlags,
    enter_notify_events_index: &'a str,
    generator_index: &'a str,
    priority: usize,
    state_name: impl Into<Cow<'a, str>>,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let enter_notify_events = if flags.contains(FNISAnimFlags::AnimObjects) {
        enter_notify_events_index
    } else if flags.contains(FNISAnimFlags::HeadTracking) {
        "#0000"
    } else {
        FNIS_AA_GLOBAL_AUTO_GEN_2526
    };
    // $-h,o|#2528|h|null|o|#2529|#2527$
    let exit_notify_events = if flags.contains(FNISAnimFlags::HeadTracking | FNISAnimFlags::AnimObjects) {
        FNIS_AA_GLOBAL_AUTO_GEN_2528
    } else if flags.contains(FNISAnimFlags::HeadTracking) {
        "#0000"
    } else if flags.contains(FNISAnimFlags::AnimObjects) {
        FNIS_AA_GLOBAL_AUTO_GEN_2529
    } else {
        FNIS_AA_GLOBAL_AUTO_GEN_2527
    };

    StateInfo {
        index: class_index,
        enter_notify_events,
        exit_notify_events,
        generator: generator_index,
        name: state_name.into(),
        ..Default::default()
    }
    .into_patch(priority)
}

/// - `state_name`:  e.g. `NPC_FNIS_km{class_index_0_id}`, `NPC_FNISpa$1/1$`
#[must_use]
pub(super) fn make_npc_root_state_info_patch<'a>(
    class_index: &'a str,
    generator_index: &'a str,
    flags: FNISAnimFlags,
    priority: usize,
    state_name: &'a str,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    // $-ac1|#2534|#2533$
    let enter_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraSet) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2534,
        false => FNIS_AA_GLOBAL_AUTO_GEN_2533,
    };
    // $-ac0|#2532|null$
    let exit_notify_events = match flags.contains(FNISAnimFlags::AnimatedCameraReset) {
        true => FNIS_AA_GLOBAL_AUTO_GEN_2532,
        false => "#0000",
    };

    StateInfo {
        index: class_index,
        enter_notify_events,
        exit_notify_events,
        generator: generator_index,
        state_id: calculate_hash(state_name),
        name: state_name.into(),
        ..Default::default()
    }
    .into_patch(priority)
}

#[must_use]
pub(super) fn new_player_synchronized_clip_generator<'a>(
    class_index: &'a str,
    event: impl Into<Cow<'a, str>>,
    generator_index: &'a str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    new_class_patch(
        class_index,
        "BSSynchronizedClipGenerator",
        priority,
        json_value!({
            "__ptr": class_index,
            "variableBindingSet": "#0000",
            "userData": 0,
            "name": event.into(), // $Ekm$ or $Epa$
            "pClipGenerator": generator_index,
            "SyncAnimPrefix": "2_", // <- Important.
            "bSyncClipIgnoreMarkPlacement": false,
            "fGetToMarkTime": 0.0,
            "fMarkErrorThreshold": 0.1,
            "bLeadCharacter": true,
            "bReorientSupportChar": true,
            "bApplyMotionFromRoot": false,
            "sAnimationBindingIndex": -1
        }),
    )
}

#[must_use]
pub(super) fn new_npc_synchronized_clip_generator<'a>(
    class_index: &'a str,
    event: impl Into<Cow<'a, str>>,
    generator_index: &'a str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    new_class_patch(
        class_index,
        "BSSynchronizedClipGenerator",
        priority,
        json_value!({
            "__ptr": class_index,
            "variableBindingSet": "#0000",
            "userData": 0,
            "name": event.into(),
            "pClipGenerator": generator_index,
            // See: https://github.com/SARDONYX-sard/serde-hkx/blob/main/crates/havok_types/src/string_ptr.rs#L181
            "SyncAnimPrefix": null, // <- XML(&#9216;) null symbol to json
            "bSyncClipIgnoreMarkPlacement": false,
            "fGetToMarkTime": 0.0,
            "fMarkErrorThreshold": 0.1,
            "bLeadCharacter": false,
            "bReorientSupportChar": true,
            "bApplyMotionFromRoot": false,
            "sAnimationBindingIndex": -1
        }),
    )
}

#[must_use]
pub(super) fn new_event_property_array<'a>(
    flags: FNISAnimFlags,
    anim_object_indexes: &[&'a str],
    class_index: &'a str,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    let mut events = vec![];

    if !flags.contains(FNISAnimFlags::HeadTracking) {
        events.push(json_value!({
            "id": 366, // HeadTrackingOff
            "payload": "#0000"
        }));
    };

    // $^ao1$ , $^ao2$
    for anim_object_index in anim_object_indexes {
        // 937: AnimObjLoad
        // 936: AnimObjDraw
        events.extend([937, 936].into_iter().map(|id| {
            json_value!({
                "id": id,
                "payload": *anim_object_index
            })
        }));
    }

    new_event_array(class_index, priority, json_value!(events))
}

#[must_use]
fn new_values_from_triggers<'a>(event_id: u64, triggers: &[Trigger<'a>]) -> Vec<simd_json::borrowed::Value<'a>> {
    let mut values: Vec<simd_json::borrowed::Value> = vec![];
    values.push(json_value!({
        "localTime": 0.0,
        "event": {
            "id": event_id,
            "payload": "#0000"
        },
        "relativeToEndOfClip": false,
        "acyclic": false,
        "isAnnotation": false
    }));

    values.par_extend(triggers.par_iter().map(|Trigger { event, time }| {
        json_value!({
            "localTime": *time, // $&TT1$
            "event": {
                "id": format!("$eventID[{event}]$"), // use Nemesis eventID variable. instead of $&TAE1$
                "payload": "#0000"
            },
            "relativeToEndOfClip": false,
            "acyclic": false,
            "isAnnotation": false
        })
    }));

    values
}

/// stateID(i32) generator?
#[must_use]
pub(super) fn calculate_hash<T: std::hash::Hash + ?Sized>(t: &T) -> i32 {
    let mut hasher = std::hash::DefaultHasher::new();
    t.hash(&mut hasher);
    (std::hash::Hasher::finish(&hasher) & (i32::MAX as u64)) as i32
}

/// This is a PairedAndKillMove and specific to `character/behaviors/0_master.xml`,
///
/// - `index`: `hkbStateMachineTransitionInfoArray` index. 0_master: `#0789`, mt_behavior: `#4038`
/// - `events`: the actual event names
///   - e.g. `["back_grab", "pa_back_grab"]`
/// - `root_events`:
///   - e.g. `["Player_FNISkm{}", "NPC_FNISkm{}"]`
///
/// # Why is this needed?
/// The array `hkbStateMachineTransitionInfoArray.transitions` links
/// an **animation event (`eventId`)** to the **root state (`toStateId`)**
/// of the state machine.
///
/// This makes it possible to trigger a transition when a kill move event fires.
///
/// ## Mapping
/// | Role   | Example `eventId`       | Example `toStateId` (root state) |
/// |--------|-------------------------|----------------------------------|
/// | Player | `back_grab`             | `Player_FNISkm{}`                |
/// | NPC    | `pa_back_grab`          | `NPC_FNISkm{}`                   |
///
/// ## Flow
/// ```text
/// anim_event (kill move)
///        │
///        ▼
///   eventId (index of hkbSBehaviorStringData.eventNames)
///        │
///        ▼
///   toStateId (root hkbStateMachineInfo.stateId)
///        │
///        ▼
///   Transition executes → state machine moves to correct root state
/// ```
///
/// ## Crash Warning
/// If `eventId` and `toStateId` do not correctly match the root
/// `hkbStateMachineInfo`, the game will **crash instantly**
/// at the moment the animation is played.
pub(super) fn new_push_transitions_seq_patch<'a, const N: usize>(
    index: &'static str,
    transition_index: &'static str,
    events: [&str; N],
    root_state_names: [&str; N],
    priority: usize,
) -> (json_path::JsonPath<'a>, ValueWithPriority<'a>) {
    let transitions: Vec<_> = events
        .iter()
        .zip(root_state_names.iter().copied())
        .map(|(event_name, root_state_name)| {
            json_value!({
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
                "transition": transition_index,
                "condition": "#0000",
                // eventId is Nemesis variable, derived from `events`
                "eventId": format!("$eventID[{event_name}]$"),
                // toStateId must match root_event (NOT the event)
                "toStateId": calculate_hash(root_state_name),
                "fromNestedStateId": 0,
                "toNestedStateId": 0,
                "priority": 0,
                "flags": "FLAG_IS_LOCAL_WILDCARD|FLAG_IS_GLOBAL_WILDCARD|FLAG_DISABLE_CONDITION"
            })
        })
        .collect();

    (
        json_path![index, "hkbStateMachineTransitionInfoArray", "transitions"],
        ValueWithPriority { patch: JsonPatch { action: Action::SeqPush, value: json_value!(transitions) }, priority },
    )
}
