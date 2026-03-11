//! # FNIS Alternative to OAR
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         └── character/                                      <- defaultmale, defaultfemale humanoid animations
//!             └── animations/
//!                 └── <fnis_mod_namespace>/                   <- this is `animations_mod_dir`
//!                     ├── xpe0_1hm_equip.hkx                  <- HKX animation file.
//!                     └── xpe0_1hm_unequip.HKX                <- HKX animation file.
//! ```
mod anim_vars;
pub mod one_syntax;

use std::borrow::Cow;

use crate::behaviors::tasks::fnis::patch_gen::alternative::gen_old_patch::one_syntax::{
    make_alt_clip_generator_patch, AltAnimMap,
};
use crate::behaviors::tasks::fnis::patch_gen::JsonPatchPairs;
use json_patch::JsonPath;
use json_patch::{Action, JsonPatch, Op, ValueWithPriority};
use rayon::prelude::*;

/// Replace the vanilla `hkbClipGenerator` indices registered as alternative animations with `hkbManualSelector`, enabling generator switching via variables.
///
/// This is replaced globally only once.
///
/// Into `meshes\actors\character\behaviors\0_master.xml`.
///
/// # Image
/// `hkbClipGenerator`(#0010) => `hkbManualSelectorGenerator`(#0010) -> `hkbClipGenerator`(#GenIndex)
///
/// # Returns
/// (one field patch, sequence field patch)
pub fn finalize_selectors<'a>(
    stage_map: AltAnimMap<'a>,
) -> (JsonPatchPairs<'a>, JsonPatchPairs<'a>) {
    let mut one_patches = Vec::new();
    let mut seq_patches = Vec::new();

    for patch in stage_map.map.into_values() {
        let clip_info = patch.clip_info;
        let group_name = clip_info.group_key;
        let priority = 0;

        // Add vanilla's `hkbClipGenerator` to the new index and place it in the selector.
        let vanilla_clip_index = format!("#FNIS_aa_global_vanilla{group_name}");
        one_patches.push(make_alt_clip_generator_patch(
            &vanilla_clip_index,
            clip_info.raw.animation_name,
            Some(clip_info.raw.triggers),
            priority,
            clip_info,
        ));

        let clip_generator_indexes: Vec<&str> = {
            // Order is important, and vanilla always comes first.
            let mut clip_generator_indexes: Vec<&str> = vec![];
            clip_generator_indexes.push(vanilla_clip_index.as_str());
            clip_generator_indexes.extend(patch.clip_generator_indexes.iter().map(|s| s.as_str()));
            clip_generator_indexes
        };

        // `hkbClipGenerator` index -> `hkbManualSelectorGenerator`
        let selector_index = format!("#FNIS_aa_global_selector{group_name}");
        one_patches.extend(make_manual_selector_generator_patch(
            clip_info.raw.ptr,
            &format!("#FNIS_aa{group_name}"),
            &selector_index,
            group_name,
            clip_generator_indexes,
            priority,
        ));

        one_patches.extend(patch.one_patches); // append ClipGenerator / TriggerArray mod patches
    }

    seq_patches.extend(new_push_anim_vars_patch(&anim_vars::ANIM_VAR_NAMES));

    (one_patches, seq_patches)
}

/// Replace `hkbClipGenerator` to `hkbManualSelectorGenerator`.
///
/// and then generators.push(prev `hkbClipGenerator` index)
/// - variable_binding_set: e.g., `#FNIS_aa_jump`
/// - group_name: e.g., `_jump`
/// - generators: alt clip indexes
#[must_use]
fn make_manual_selector_generator_patch<'a>(
    vanilla_clip_index: &str,
    variable_binding_set: &str,
    id: &str,
    group_name: &str,
    generators: Vec<&str>,
    priority: usize,
) -> [(JsonPath<'a>, ValueWithPriority<'a>); 2] {
    [
        (
            vec![
                Cow::Owned(variable_binding_set.to_string()),
                Cow::Borrowed("hkbVariableBindingSet"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Add },
                    value: simd_json::json_typed!(borrowed, {
                            "__ptr": variable_binding_set,
                            "bindings": [
                                {
                                    "memberPath": "selectedGeneratorIndex",
                                    // select ANIM_VARS variable
                                    "variableIndex": format!("$variableName[FNISaa{group_name}]$"), // use Nemesis variable e.g., `FNISaa_jump`
                                    "bitIndex": -1,
                                    "bindingType": "BINDING_TYPE_VARIABLE"
                                }
                            ],
                            "indexOfBindingToEnable": -1
                    }),
                },
                priority,
            },
        ),
        (
            vec![
                Cow::Owned(vanilla_clip_index.to_string()),
                Cow::Borrowed("hkbManualSelectorGenerator"),
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::Pure { op: Op::Replace },
                    value: simd_json::json_typed!(borrowed, {
                        "__ptr": vanilla_clip_index,
                        "variableBindingSet": variable_binding_set,
                        "userData": 0,
                        "name": format!("FNIS_{id}_{group_name}_MSG"),
                        "generators": generators,
                        "selectedGeneratorIndex": 0,
                        "currentGeneratorIndex": 0
                    }),
                },
                priority,
            },
        ),
    ]
}

/// This variable likely needs to be registered below to `0_master.xml`.
///
/// - Kind: Seq patch
///
/// - `hkbBehaviorGraphStringData.variableNames`
/// - `hkbVariableValueSet.wordVariableValues`
/// - `hkbBehaviorGraphData.variableInfos`(as [i32])
fn new_push_anim_vars_patch<'a>(values: &[&'a str]) -> [(JsonPath<'a>, ValueWithPriority<'a>); 3] {
    use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::DEFAULT_FEMALE;

    let string_data_index = DEFAULT_FEMALE.master_string_data_index;
    let variable_index = DEFAULT_FEMALE.master_value_set_index;
    let behavior_graph_index = DEFAULT_FEMALE.master_behavior_graph_index;
    let priority = 0;

    [
        (
            json_patch::json_path![
                string_data_index,
                "hkbBehaviorGraphStringData",
                "variableNames",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(borrowed, values),
                },
                priority,
            },
        ),
        (
            json_patch::json_path![variable_index, "hkbVariableValueSet", "wordVariableValues"],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(
                        borrowed,
                        values
                            .par_iter()
                            .map(|_| simd_json::json_typed!(borrowed, {
                                "value": 0
                            }))
                            .collect::<Vec<_>>()
                    ),
                },
                priority,
            },
        ),
        (
            json_patch::json_path![
                behavior_graph_index,
                "hkbBehaviorGraphData",
                "variableInfos",
            ],
            ValueWithPriority {
                patch: JsonPatch {
                    action: Action::SeqPush,
                    value: simd_json::json_typed!(
                        borrowed,
                        values
                            .par_iter()
                            .map(|_| {
                                simd_json::json_typed!(borrowed, {
                                    "role": {
                                        "role": "ROLE_DEFAULT",
                                        "flags": "0"
                                    },
                                    "type": "VARIABLE_TYPE_INT32"
                                })
                            })
                            .collect::<Vec<_>>()
                    ),
                },
                priority,
            },
        ),
    ]
}
