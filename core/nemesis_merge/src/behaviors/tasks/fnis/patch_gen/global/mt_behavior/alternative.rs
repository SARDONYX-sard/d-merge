//! For `character/behaviors/mt_behavior.xml` 1 file patch

use crate::behaviors::tasks::fnis::patch_gen::JsonPatchPairs;
use json_patch::{json_path, Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;

/// FNIS XML(name="#5218") - `HeadTrackingOn`
pub(crate) const FNIS_AA_MT_AUTO_GEN_5218: &str = "#FNIS_aa_global_auto_gen5218";

/// FNIS XML(name="#5219") - `HeadTrackingOff`
pub(crate) const FNIS_AA_MT_AUTO_GEN_5219: &str = "#FNIS_aa_global_auto_gen5219";

/// FNIS XML(name="#5220") - `StartAnimatedCamera`
pub(crate) const FNIS_AA_MT_AUTO_GEN_5220: &str = "#FNIS_aa_global_auto_gen5220";

/// FNIS XML(name="#5221") - `EndAnimatedCamera`
pub(crate) const FNIS_AA_MT_AUTO_GEN_5221: &str = "#FNIS_aa_global_auto_gen5221";

/// FNIS XML(name="#5222") - `ClipTriggerArray`
pub(crate) const FNIS_AA_MT_CLIP_TRIGGER_5222: &str = "#FNIS_aa_global_auto_gen5222";

/// FNIS XML(name="#5233") - `FNIS_BlendTransition1`(`#$:BlendTransition+&bl$`)
/// - duration: 0.6
pub(crate) const FNIS_BA_BLEND_TRANSITION_5231: &str = "#FNIS_aa_global_auto_gen5231";
/// FNIS XML(name="#5232") - `FNIS_BlendTransition2`
/// - duration: 1.2
pub(crate) const FNIS_BA_BLEND_TRANSITION_5232: &str = "#FNIS_aa_global_auto_gen5232";

/// Generate the Havok class of `character/behaviors/mt_behavior.xml`.
///
/// These are classes that are generated only once per file.
///
/// # Note
/// Generated for Alternate Animations(FNIS_aa).
/// However, they are actually also reused in Offset Arm Animations, so they must be generated.
///
/// See: `FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\mt_behavior_TEMPLATE.txt`
pub(super) fn new_mt_global_patch<'a>(priority: usize) -> JsonPatchPairs<'a> {
    let single_events: [(&'static str, i32); 4] = [
        (FNIS_AA_MT_AUTO_GEN_5218, 18),  // HeadTrackingOn
        (FNIS_AA_MT_AUTO_GEN_5219, 20),  // HeadTrackingOff
        (FNIS_AA_MT_AUTO_GEN_5220, 461), // StartAnimatedCamera
        (FNIS_AA_MT_AUTO_GEN_5221, 462), // EndAnimatedCamera
    ];

    let mut patches: JsonPatchPairs<'a> = single_events
        .par_iter()
        .map(|&(class_index, id)| {
            (
                json_path![class_index, "hkbStateMachineEventPropertyArray"],
                ValueWithPriority {
                    patch: JsonPatch {
                        action: Action::Pure { op: Op::Add },
                        value: simd_json::json_typed!(borrowed, {
                            "__ptr": class_index,
                            "events": [
                                {
                                    "id": id,
                                    "payload": "#0000",
                                }
                            ]
                        }),
                    },
                    priority,
                },
            )
        })
        .collect();

    // ClipTriggerArray (#5222)
    patches.push((
        json_path![FNIS_AA_MT_CLIP_TRIGGER_5222, "hkbClipTriggerArray"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_MT_CLIP_TRIGGER_5222,
                    "triggers": [
                        {
                            "localTime": -0.2,
                            "event": { "id": 91, "payload": "#0000" }, // ChairNextClip
                            "relativeToEndOfClip": true,
                            "acyclic": false,
                            "isAnnotation": false
                        },
                        {
                            "localTime": 1.0,
                            "event": { "id": 20, "payload": "#0000" }, // HeadTrackingOff
                            "relativeToEndOfClip": false,
                            "acyclic": false,
                            "isAnnotation": false
                        },
                        {
                            "localTime": -1.0,
                            "event": { "id": 18, "payload": "#0000" }, // HeadTrackingOn
                            "relativeToEndOfClip": false,
                            "acyclic": false,
                            "isAnnotation": false
                        }
                    ]
                }),
            },
            priority,
        },
    ));

    patches.push( (
        json_path![FNIS_BA_BLEND_TRANSITION_5231, "hkbBlendingTransitionEffect"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_BA_BLEND_TRANSITION_5231,
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": "FNIS_BlendTransition1",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_CONTINUE_IF_CYCLIC_BLEND_IF_ACYCLIC",
                    "eventMode": "EVENT_MODE_PROCESS_ALL",
                    "duration": 0.6,
                    "toGeneratorStartTimeFraction": 0.0,
                    "flags": "FLAG_IGNORE_FROM_WORLD_FROM_MODEL",
                    "endMode": "END_MODE_NONE",
                    "blendCurve": "BLEND_CURVE_SMOOTH"
                }),
            },
            priority,
        },
    ));
    patches.push( (
        json_path![FNIS_BA_BLEND_TRANSITION_5232, "hkbBlendingTransitionEffect"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_BA_BLEND_TRANSITION_5232,
                    "variableBindingSet": "#0000",
                    "userData": 0,
                    "name": "FNIS_BlendTransition2",
                    "selfTransitionMode": "SELF_TRANSITION_MODE_CONTINUE_IF_CYCLIC_BLEND_IF_ACYCLIC",
                    "eventMode": "EVENT_MODE_PROCESS_ALL",
                    "duration": 1.2,
                    "toGeneratorStartTimeFraction": 0.0,
                    "flags": "FLAG_IGNORE_FROM_WORLD_FROM_MODEL",
                    "endMode": "END_MODE_NONE",
                    "blendCurve": "BLEND_CURVE_SMOOTH"
                }),
            },
            priority,
        },
    ));

    patches
}
