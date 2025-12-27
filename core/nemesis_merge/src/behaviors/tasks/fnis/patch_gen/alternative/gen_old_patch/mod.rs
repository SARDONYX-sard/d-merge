//! # FNIS Alternative to OAR
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         └── character/                                      <- defaultmale, defaultfemale humanoid animations
//!             └── animations/
//!                 └── <fnis_mod_namespace>/                   <- this is `animations_mod_dir`
//!                     ├── FNIS_<namespace>_toOAR.json         <- FNIS alt anim to OAR override config file.(optional)
//!                     ├── xpe0_1hm_equip.hkx                  <- HKX animation file.
//!                     └── xpe0_1hm_unequip.HKX                <- HKX animation file.
//! ```
mod gen_for_one_file;

use std::borrow::Cow;
use std::collections::HashMap;

use crate::behaviors::tasks::fnis::patch_gen::alternative::gen_old_patch::gen_for_one_file::make_alt_clip_generator_patch;
use crate::behaviors::tasks::fnis::patch_gen::alternative::gen_old_patch::gen_for_one_file::ClipBuildResult;
use crate::behaviors::tasks::fnis::patch_gen::JsonPatchPairs;

use json_patch::{Action, JsonPatch, Op, ValueWithPriority};

pub fn finalize_selectors<'a>(
    stage_map: HashMap<&'a str, ClipBuildResult<'a>>,
) -> JsonPatchPairs<'a> {
    let mut patches = Vec::new();

    for (anim_name, result) in stage_map {
        let id = &result.owned_data.namespace;
        let priority = result.owned_data.priority;
        let group_name = result.clip_info.group_key;

        // Add vanilla's `hkbClipGenerator` to the new index and place it in the selector.
        let vanilla_clip_generator = result.owned_data.next_class_name_attribute();
        patches.push(make_alt_clip_generator_patch(
            &vanilla_clip_generator,
            result.clip_info.raw.animation_name,
            Some(result.clip_info.raw.triggers),
            priority,
            result.clip_info,
        ));

        let mut clip_generator_indexes: Vec<&str> = vec![];
        // Order is important, and vanilla always comes first.
        clip_generator_indexes.push(vanilla_clip_generator.as_str());
        clip_generator_indexes.extend(result.clip_generator_indexes.iter().map(|s| s.as_str()));

        // Replace the location that was vanilla's `hkbClipGenerator` with a Selector to enable animation changes via variables.
        let selector_index = result.clip_info.raw.ptr; // `hkbClipGenerator` index -> `hkbManualSelectorGenerator`
        patches.push(make_manual_selector_generator_patch(
            selector_index,
            &format!("#FNIS_aa_{group_name}"),
            id,
            group_name,
            clip_generator_indexes,
            priority,
        ));

        // append ClipGenerator / TriggerArray patches
        patches.extend(result.one_patches);
    }

    patches
}

/// Replace `hkbClipGenerator` to `hkbManualSelectorGenerator`.
///
/// and then generators.push(prev `hkbClipGenerator` index)
/// - variable_binding_set: e.g., `#FNIS_aa_jump`
/// - group_name: e.g., `_jump`
/// - generators: alt clip indexes
#[must_use]
fn make_manual_selector_generator_patch<'a>(
    class_index: &str,
    variable_binding_set: &str,
    id: &str,
    group_name: &str,
    generators: Vec<&str>,
    priority: usize,
) -> (Vec<Cow<'a, str>>, ValueWithPriority<'a>) {
    (
        vec![
            Cow::Owned(class_index.to_string()),
            Cow::Borrowed("hkbManualSelectorGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                action: Action::Pure { op: Op::Replace },
                value: simd_json::json_typed!(borrowed, {
                    "__ptr": class_index,
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
    )
}
