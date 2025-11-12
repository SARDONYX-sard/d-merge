//! For `character/behaviors/mt_behavior.xml` 1 file patch
//! - `FNIS Furniture Begin (ROOT)`
use crate::behaviors::tasks::fnis::patch_gen::JsonPatchPairs;
use json_patch::{json_path, Action, JsonPatch, Op, ValueWithPriority};

/// FNIS XML(name="#5200") - `hkbStateMachineStateInfo`
const FNIS_FU_MT_5200: &str = "#FNIS_fu_global_auto_gen5200";
/// FNIS XML(name="#5201") - `hkbStateMachineTransitionInfoArray`
const FNIS_FU_MT_5201: &str = "#FNIS_fu_global_auto_gen5201";
/// FNIS XML(name="#5202") - `hkbModifierGenerator`
const FNIS_FU_MT_5202: &str = "#FNIS_fu_global_auto_gen5202";
/// FNIS XML(name="#5203") - `hkbModifierList`
const FNIS_FU_MT_5203: &str = "#FNIS_fu_global_auto_gen5203";
/// FNIS XML(name="#5204") - `BSIsActiveModifier`
const FNIS_FU_MT_5204: &str = "#FNIS_fu_global_auto_gen5204";
/// FNIS XML(name="#5205") - `hkbVariableBindingSet`
const FNIS_FU_MT_5205: &str = "#FNIS_fu_global_auto_gen5205";
/// FNIS XML(name="#5206") - `BSModifyOnceModifier`
const FNIS_FU_MT_5206: &str = "#FNIS_fu_global_auto_gen5206";
/// FNIS XML(name="#5207") - `hkbEvaluateExpressionModifier`
const FNIS_FU_MT_5207: &str = "#FNIS_fu_global_auto_gen5207";
/// FNIS XML(name="#5208") - `hkbExpressionDataArray`
const FNIS_FU_MT_5208: &str = "#FNIS_fu_global_auto_gen5208";
/// FNIS XML(name="#5209") - `BSIsActiveModifier`
const FNIS_FU_MT_5209: &str = "#FNIS_fu_global_auto_gen5209";
/// FNIS XML(name="#5210") - `hkbVariableBindingSet`
const FNIS_FU_MT_5210: &str = "#FNIS_fu_global_auto_gen5210";
/// FNIS XML(name="#5211") - `BSEventOnDeactivateModifier`
const FNIS_FU_MT_5211: &str = "#FNIS_fu_global_auto_gen5211";
/// FNIS XML(name="#5212") - `hkbStateMachine`
const FNIS_FU_MT_5212: &str = "#FNIS_fu_global_auto_gen5212";
/// FNIS XML(name="#5213") - `hkbVariableBindingSet`
const FNIS_FU_MT_5213: &str = "#FNIS_fu_global_auto_gen5213";
/// FNIS XML(name="#5214") - `hkbBlendingTransitionEffect`
const FNIS_FU_MT_5214: &str = "#FNIS_fu_global_auto_gen5214";
/// FNIS XML(name="#5215") - `hkbBlendingTransitionEffect`
const FNIS_FU_MT_5215: &str = "#FNIS_fu_global_auto_gen5215";
/// FNIS XML(name="#5216") - `hkbBlendingTransitionEffect`
const FNIS_FU_MT_5216: &str = "#FNIS_fu_global_auto_gen5216";

/// Generate the Havok class of `character/behaviors/mt_behavior.xml`.
///
/// These are classes that are generated only once per file.
///
/// # Note
/// Generated for alternative animations(FNIS_aa).
/// However, they are actually also reused in Offset Arm Animations, so they must be generated.
///
/// See: `FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt`
pub(super) fn new_mt_global_patch<'a>(
    patches: (&mut JsonPatchPairs<'a>, &mut JsonPatchPairs<'a>),
    anim_groups_states: Vec<String>,
    priority: usize,
) {
    let (one_patches, seq_patches) = patches;

    seq_patches.push((
        json_path!["#5195", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::SeqPush,
                value: simd_json::json_typed!(borrowed, [FNIS_FU_MT_5200]),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5200, "hkbStateMachineStateInfo"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5200,
                    "variableBindingSet": "#0000",
                    "listeners": [],
                    "enterNotifyEvents": "#0000",
                    "exitNotifyEvents": "#0000",
                    "transitions": FNIS_FU_MT_5201,
                    "generator": FNIS_FU_MT_5202,
                    "name": "FNIS_FurnitureState",
                    "stateId": 555, // FIXME :? Be careful about ID conflicts with other mods. -> Should we use `calc_id(FNIS_FU_MT_5200)`?
                    "probability": 1.0,
                    "enable": true
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5201, "hkbStateMachineTransitionInfoArray"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5201,
                    "transitions": [
                        {
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
                            "transition": FNIS_FU_MT_5214,
                            "condition": "#0000",
                            "eventId": 5,          // IdleFurnitureExit
                            "toStateId": 0,
                            "fromNestedStateId": 0,
                            "toNestedStateId": 0,
                            "priority": 0,
                            "flags": "FLAG_DISABLE_CONDITION"
                        },
                        {
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
                            "transition": FNIS_FU_MT_5215,
                            "condition": "#0000",
                            "eventId": 473,        // IdleFurnitureExitSlow
                            "toStateId": 0,
                            "fromNestedStateId": 0,
                            "toNestedStateId": 0,
                            "priority": 0,
                            "flags": "FLAG_DISABLE_CONDITION"
                        }
                    ]
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5202, "hkbModifierGenerator"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5202,
                    "variableBindingSet": "#0000",
                    "userData": 1,
                    "name": "FNIS_Furniture_MG",
                    "modifier": FNIS_FU_MT_5203,
                    "generator": FNIS_FU_MT_5212
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5203, "hkbModifierList"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5203,
                    "variableBindingSet": "#0000",
                    "userData": 1,
                    "name": "FNIS_Furniture_ModifierList",
                    "enable": true,
                    "modifiers": [
                        FNIS_FU_MT_5204,
                        FNIS_FU_MT_5206,
                        FNIS_FU_MT_5209,
                        FNIS_FU_MT_5211
                    ]
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5204, "BSIsActiveModifier"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5204,
                    "variableBindingSet": FNIS_FU_MT_5205,
                    "userData": 2,
                    "name": "bHeadTrackSpine_IsActiveModifier",
                    "enable": true,
                    "bIsActive0": false,
                    "bInvertActive0": true,
                    "bIsActive1": false,
                    "bInvertActive1": true,
                    "bIsActive2": false,
                    "bInvertActive2": true,
                    "bIsActive3": false,
                    "bInvertActive3": true,
                    "bIsActive4": false,
                    "bInvertActive4": true
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5205, "hkbVariableBindingSet"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5205,
                    "bindings": [
                        {
                            "memberPath": "bIsActive0",
                            "variableIndex": 41,   // bHeadTrackSpine
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        },
                        {
                            "memberPath": "bIsActive1",
                            "variableIndex": 63,   // bHumanoidFootIKEnable
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5206, "BSModifyOnceModifier"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5206,
                    "variableBindingSet": "#0000",
                    "userData": 3,
                    "name": "SetSneakFalse",
                    "enable": true,
                    "pOnActivateModifier": FNIS_FU_MT_5207,
                    "pOnDeactivateModifier": "#0000"
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5207, "hkbEvaluateExpressionModifier"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5207,
                    "variableBindingSet": "#0000",
                    "userData": 2,
                    "name": "SetSneakFalse_EEM",
                    "enable": true,
                    "expressions": FNIS_FU_MT_5208
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5208, "hkbExpressionDataArray"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5208,
                    "expressionsData": [
                        {
                            "expression": "iIsInSneak = 0",
                            "assignmentVariableIndex": -1,
                            "assignmentEventIndex": -1,
                            "eventMode": "EVENT_MODE_SEND_ONCE"
                        }
                    ]
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5209, "BSIsActiveModifier"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5209,
                    "variableBindingSet": FNIS_FU_MT_5210,
                    "userData": 2,
                    "name": "bEquipOK_IsNotActive",
                    "enable": true,
                    "bIsActive0": false,
                    "bInvertActive0": true,
                    "bIsActive1": false,
                    "bInvertActive1": false,
                    "bIsActive2": false,
                    "bInvertActive2": false,
                    "bIsActive3": false,
                    "bInvertActive3": false,
                    "bIsActive4": false,
                    "bInvertActive4": false
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5210, "hkbVariableBindingSet"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5210,
                    "bindings": [
                        {
                            "memberPath": "bIsActive0",
                            "variableIndex": 51,  // bEquipOK
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5211, "BSEventOnDeactivateModifier"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5211,
                    "variableBindingSet": "#0000",
                    "userData": 3,
                    "name": "IdleChairGetUp_DeactivateMod",
                    "enable": true,
                    "event": {
                        "id": 15,          // IdleChairGetUp
                        "payload": "#0000"
                    }
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5212, "hkbStateMachine"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5212,
                    "variableBindingSet": FNIS_FU_MT_5213,
                    "userData": 0,
                    "name": "FNIS_Furniture_BehaviorGraph",
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
                    "selfTransitionMode": "SELF_TRANSITION_MODE_FORCE_TRANSITION_TO_START_STATE",
                    "states": anim_groups_states,
                    "wildcardTransitions": "#0000"
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5213, "hkbVariableBindingSet"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5213,
                    "bindings": [
                        {
                            "memberPath": "isActive",
                            "variableIndex": 1,  // bAnimationDriven
                            "bitIndex": -1,
                            "bindingType": "BINDING_TYPE_VARIABLE"
                        }
                    ],
                    "indexOfBindingToEnable": -1
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5214, "hkbBlendingTransitionEffect"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5214,
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": "DefaultBlend_FromAnimationDriven",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_BLEND",
                    "eventMode": "EVENT_MODE_PROCESS_ALL",
                    "duration": 0.2,
                    "toGeneratorStartTimeFraction": 0.0,
                    "flags": "FLAG_IGNORE_TO_WORLD_FROM_MODEL",
                    "endMode": "END_MODE_NONE",
                    "blendCurve": "BLEND_CURVE_SMOOTH"
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5215, "hkbBlendingTransitionEffect"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5215,
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": "SlowBlendTransitionFromAnimationDriven",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_BLEND",
                    "eventMode": "EVENT_MODE_PROCESS_ALL",
                    "duration": 1.0,
                    "toGeneratorStartTimeFraction": 0.0,
                    "flags": "FLAG_IGNORE_TO_WORLD_FROM_MODEL",
                    "endMode": "END_MODE_NONE",
                    "blendCurve": "BLEND_CURVE_SMOOTH"
                }),
            },
            priority,
        },
    ));

    one_patches.push((
        json_path![FNIS_FU_MT_5216, "hkbBlendingTransitionEffect"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_FU_MT_5216,
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": "0.5secondBlendToFurniture",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_CONTINUE_IF_CYCLIC_BLEND_IF_ACYCLIC",
                    "eventMode": "EVENT_MODE_PROCESS_ALL",
                    "duration": 0.5,
                    "toGeneratorStartTimeFraction": 0.0,
                    "flags": 0,
                    "endMode": "END_MODE_NONE",
                    "blendCurve": "BLEND_CURVE_SMOOTH"
                }),
            },
            priority,
        },
    ));
}
