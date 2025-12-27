use crate::behaviors::tasks::fnis::patch_gen::JsonPatchPairs;
use json_patch::{json_path, Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;

/// FNIS XML(name="#2526") - `HeadTrackingOff`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2526: &str = "#FNIS_aa_global_auto_gen2526";

/// FNIS XML(name="#2527") - `HeadTrackingOn`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2527: &str = "#FNIS_aa_global_auto_gen2527";

/// FNIS XML(name="#2528") - `AnimObjectUnequip`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2528: &str = "#FNIS_aa_global_auto_gen2528";

/// FNIS XML(name="#2529") - `Multi (HeadTrackingOn + AnimObjectUnequip)`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2529: &str = "#FNIS_aa_global_auto_gen2529";

/// FNIS XML(name="#2530") - `StartAnimatedCamera`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2530: &str = "#FNIS_aa_global_auto_gen2530";

/// FNIS XML(name="#2531") - `StringEventPayload (Camera3rd [Cam3])`
pub(crate) const FNIS_AA_STRING_PAYLOAD_2531: &str = "#FNIS_aa_global_auto_gen2531";

/// FNIS XML(name="#2532") - `EndAnimatedCamera`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2532: &str = "#FNIS_aa_global_auto_gen2532";

/// FNIS XML(name="#2533") - `PairedKillTarget`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2533: &str = "#FNIS_aa_global_auto_gen2533";

/// FNIS XML(name="#2534") - `Multi (StartAnimatedCamera + PairedKillTarget)`
pub(crate) const FNIS_AA_GLOBAL_AUTO_GEN_2534: &str = "#FNIS_aa_global_auto_gen2534";

/// Generate the Havok class corresponding to the options flags in FNIS_*_List.txt.
///
/// # Note
/// The classes generated here are used in `character/behaviors/0_master.xml`
/// and are generated for Alternate Animations(FNIS_aa).
///
/// However, they are actually also reused in PairedAndKillMoves, so they must be generated.
///
/// See: `FNIS Behavior SE 7.6\tools\GenerateFNIS_for_Users\templates\0_master_TEMPLATE.txt`
pub fn new_global_master_patch<'a>(priority: usize) -> JsonPatchPairs<'a> {
    // single event (#2526, #2527, #2528, #2530, #2532, #2533)
    let single_events: [(&'static str, i32, Option<&'static str>); 6] = [
        (FNIS_AA_GLOBAL_AUTO_GEN_2526, 366, None), // HeadTrackingOff
        (FNIS_AA_GLOBAL_AUTO_GEN_2527, 367, None), // HeadTrackingOn
        (FNIS_AA_GLOBAL_AUTO_GEN_2528, 543, None), // AnimObjectUnequip
        (FNIS_AA_GLOBAL_AUTO_GEN_2530, 1061, Some("#2531")), // StartAnimatedCamera
        (FNIS_AA_GLOBAL_AUTO_GEN_2532, 1062, None), // EndAnimatedCamera
        (FNIS_AA_GLOBAL_AUTO_GEN_2533, 915, None), // PairedKillTarget
    ];

    let mut patches: JsonPatchPairs<'a> = single_events
        .par_iter()
        .map(|&(class_index, id, payload)| {
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
                                    "payload": payload.unwrap_or("#0000"),
                                }
                            ]
                        }),
                    },
                    priority,
                },
            )
        })
        .collect();

    let multi_event_2529 = (
        json_path![
            FNIS_AA_GLOBAL_AUTO_GEN_2529,
            "hkbStateMachineEventPropertyArray"
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_GLOBAL_AUTO_GEN_2529,
                    "events": [
                        { "id": 367, "payload": "#0000" }, // HeadTrackingOn
                        { "id": 543, "payload": "#0000" }, // AnimObjectUnequip
                    ]
                }),
            },
            priority,
        },
    );
    let multi_event_2534 = (
        json_path![
            FNIS_AA_GLOBAL_AUTO_GEN_2534,
            "hkbStateMachineEventPropertyArray"
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_GLOBAL_AUTO_GEN_2534,
                    "events": [
                        { "id": 1061, "payload": FNIS_AA_STRING_PAYLOAD_2531 }, // StartAnimatedCamera
                        { "id": 915,  "payload": "#0000"   }, // PairedKillTarget
                    ]
                }),
            },
            priority,
        },
    );
    let camera_event_2531 = (
        json_path![FNIS_AA_STRING_PAYLOAD_2531, "hkbStringEventPayload"],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Add },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": FNIS_AA_STRING_PAYLOAD_2531,
                    "data": "Camera3rd [Cam3]"
                }),
            },
            priority,
        },
    );
    patches.par_extend([multi_event_2529, multi_event_2534, camera_event_2531]);

    patches
}
