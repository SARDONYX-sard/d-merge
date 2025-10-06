mod gen_list_patch;
pub mod generated_behaviors;
mod kill_move;
mod pair;

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::Path;

use dashmap::DashSet;
use json_patch::{json_path, JsonPatch, JsonPath, Op, OpRangeKind, ValueWithPriority};
use rayon::iter::Either;
use rayon::prelude::*;
use simd_json::json_typed;
use snafu::ResultExt;
use winnow::Parser;

use crate::behaviors::tasks::adsf::AdsfPatch;
use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::parse_fnis_list;
use crate::behaviors::tasks::fnis::patch_gen::gen_list_patch::generate_patch;
use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::BehaviorEntry;
use crate::behaviors::tasks::fnis::patch_gen::generated_behaviors::DEFAULT_FEMALE;
use crate::behaviors::tasks::patches::types::{
    BehaviorStringDataMap, BorrowedPatches, RawBorrowedPatches,
};
use crate::behaviors::tasks::templates::key::TemplateKey;
use crate::config::{ReportType, StatusReportCounter, StatusReporterFn};
use crate::errors::{Error, FailedParseFnisModListSnafu};

/// For Seq patch
pub(crate) const PUSH_OP: OpRangeKind = OpRangeKind::Seq(json_patch::OpRange {
    op: Op::Add,
    range: 9998..9999,
});

pub fn collect_borrowed_patches<'a>(
    mods_patches: &'a [OwnedFnisInjection],
    status_reporter: &'a StatusReporterFn,
) -> (BorrowedPatches<'a>, Vec<AdsfPatch<'a>>, Vec<Error>) {
    let raw_borrowed_patches = RawBorrowedPatches::default();
    let template_keys = DashSet::new();
    let variable_class_map = BehaviorStringDataMap::new(); // TODO: Change to compile time phf map.

    let reporter = StatusReportCounter::new(
        status_reporter,
        ReportType::GeneratingFnisPatches,
        mods_patches.len(),
    );

    let (adsf_patches, errors): (Vec<_>, Vec<_>) = mods_patches
        .par_iter()
        .map(|owed_ref_one_mod| {
            reporter.increment();

            // Push Mod Root behavior to master xml
            {
                let master_template_key = owed_ref_one_mod
                    .behavior_entry
                    .to_master_behavior_template_key();

                template_keys.insert(master_template_key.clone());

                let (one_gen, one_state_info, seq_state) =
                    new_injectable_mod_root_behavior(owed_ref_one_mod);

                let entry = raw_borrowed_patches
                    .0
                    .entry(master_template_key)
                    .or_default();
                entry.one.insert(one_gen.0, one_gen.1);
                entry.one.insert(one_state_info.0, one_state_info.1);
                entry.seq.insert(seq_state.0, seq_state.1);
            }

            let list = match parse_fnis_list
                .parse(&owed_ref_one_mod.list_content)
                .map_err(|e| serde_hkx::errors::readable::ReadableError::from_parse(e))
                .with_context(|_| FailedParseFnisModListSnafu {
                    path: Path::new(&format!(
                        "meshes/{}/animations/{}/FNIS_*_List.txt",
                        owed_ref_one_mod.behavior_entry.base_dir, owed_ref_one_mod.namespace
                    ))
                    .to_path_buf(),
                }) {
                Ok(list) => list,
                Err(err) => return Either::Right(err),
            };

            #[cfg(feature = "tracing")]
            {
                let base_dir = owed_ref_one_mod.behavior_entry.base_dir;
                let namespace = &owed_ref_one_mod.namespace;
                tracing::debug!(
                    "meshes/{base_dir}/animations/{namespace}/FNIS_*_List.txt: \n{list:#?}"
                );
            }

            let (animations, adsf_patches) = generate_patch(owed_ref_one_mod, list);
            // NOTE: The addition of animations has been tested to work in any order, but just to be safe.
            let animations: HashSet<_> = animations.into_iter().collect();
            let mut animations: Vec<_> = animations.into_iter().collect();
            animations.par_sort_unstable();

            // Push One Mod animations
            if !animations.is_empty() {
                insert_anim_seq_patch(
                    &animations,
                    owed_ref_one_mod.behavior_entry,
                    owed_ref_one_mod.priority,
                    &raw_borrowed_patches,
                    &template_keys,
                );

                if owed_ref_one_mod.behavior_entry.is_humanoid() {
                    insert_anim_seq_patch(
                        &animations,
                        &DEFAULT_FEMALE,
                        owed_ref_one_mod.priority,
                        &raw_borrowed_patches,
                        &template_keys,
                    );
                } else if owed_ref_one_mod.behavior_entry.behavior_object == "draugr" {
                    // # Why need this?
                    // It seems draugr must have the animations path added to both draugr.xml and
                    // draugr_skeleton.xml (information from the FNIS Creature pack's behavior object).
                    const DRAUGR_SKELETON: BehaviorEntry = BehaviorEntry {
                        behavior_object: "draugr",
                        base_dir: "actors/draugr",
                        default_behavior: "characterskeleton/draugr_skeleton.bin",
                        default_behavior_index: "#0024",
                        master_behavior: "behaviors/draugrbehavior.bin",
                        master_behavior_index: "#2026",
                    };

                    insert_anim_seq_patch(
                        &animations,
                        &DRAUGR_SKELETON,
                        owed_ref_one_mod.priority,
                        &raw_borrowed_patches,
                        &template_keys,
                    );
                }
            };

            Either::Left(adsf_patches)
        })
        .collect();

    let adsf_patches: Vec<_> = adsf_patches.into_par_iter().flatten().collect();

    (
        BorrowedPatches {
            template_keys,
            borrowed_patches: raw_borrowed_patches,
            behavior_string_data_map: variable_class_map,
        },
        adsf_patches,
        errors,
    )
}

/// Register a mod's root behavior (`behaviors\FNIS_<namespace>_Behavior.hkx`)
/// into `meshes\actors\character\behaviors\0_master.xml`.
///
/// - `behavior_id`: Unique identifier for the behavior (e.g., `#<namespace>${index}`).
/// - `behavior_path`: Path to the behavior file used in `hkbBehaviorReferenceGenerator.behavior_name`.
fn new_injectable_mod_root_behavior<'a>(
    owned_data: &'a OwnedFnisInjection,
) -> (
    (JsonPath<'a>, ValueWithPriority<'a>),
    (JsonPath<'a>, ValueWithPriority<'a>),
    (JsonPath<'a>, ValueWithPriority<'a>),
) {
    // NOTE: To learn the additional method, I enabled only one FNIS mod and ran it, then read the XML in tools/*/`temporary_logs`.
    let namespace = owned_data.namespace.as_str();
    let priority = owned_data.priority;
    let behavior_path = owned_data.behavior_path.as_str();
    let new_generator_index = owned_data.next_class_name_attribute();
    let new_root_state_info_index = owned_data.next_class_name_attribute();
    let master_index = owned_data.behavior_entry.master_behavior_index;

    let one_state_info = (
        vec![
            Cow::Owned(new_root_state_info_index.clone()),
            Cow::Borrowed("hkbStateMachineStateInfo"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                        "__ptr": new_root_state_info_index,
                        "variableBindingSet": "#0000",
                        "listeners": [],
                        "enterNotifyEvents": "#0000",
                        "exitNotifyEvents": "#0000",
                        "transitions": "#0000",
                        "generator": new_generator_index,
                        "name": format!("FNIS_State{priority}"),
                        "stateId": 1000 + priority, // FIXME?
                        "probability": 1.0,
                        "enable": true
                }),
            },
            priority,
        },
    );

    let one_gen = (
        vec![
            Cow::Owned(new_generator_index.clone()),
            Cow::Borrowed("hkbBehaviorReferenceGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": new_generator_index,
                    "variableBindingSet": "#0000", // null
                    "userData": 0,

                    // NOTE: FNIS_ROOT_BFR{index}: In FNIS, it's actually the ordering index.
                    // but here we use priority instead.
                    "name": format!("FNIS_ROOT_BFR_{namespace}_{priority}"), // StringPtr
                    "behaviorName": behavior_path, // StringPtr
                }),
            },
            priority,
        },
    );

    let seq_state = (
        json_path![master_index, "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, [new_root_state_info_index]),
            },
            priority,
        },
    );

    (one_gen, one_state_info, seq_state)
}

/// Insert a new animation sequence patch into the borrowed patches map.
///
/// Create an additional patch for the animations for one of the following template files.
/// - `#0029`
///    - `meshes/actors/character/_1stperson/firstperson.xml`
///    - `meshes/actors/character/default_female/defaultfemale.xml`
///    - `meshes/actors/character/defaultmale/defaultmale.xml`
///
/// # Note
/// `animations`: Windows path to the Animations dir containing files within `meshes/actors/character/animations/<FNIS one mod namespace>/*.hkx`.
///
/// - sample animations
/// ```txt
/// [
///     "Animations\<FNIS one mod namespace>\sample.hkx",
///     "Animations\<FNIS one mod namespace>\sample1.hkx"
/// ]
/// ```
fn insert_anim_seq_patch<'a>(
    animations: &[String],
    behavior_entry: &BehaviorEntry,
    priority: usize,
    raw_borrowed_patches: &RawBorrowedPatches<'a>,
    template_keys: &DashSet<TemplateKey<'static>>,
) {
    let behavior_key = behavior_entry.to_default_behavior_template_key();

    template_keys.insert(behavior_key.clone());

    let (json_path, patch) = {
        let index = behavior_entry.default_behavior_index;
        (
            json_path![index, "hkbCharacterStringData", "animationNames"],
            ValueWithPriority {
                patch: JsonPatch {
                    op: PUSH_OP,
                    value: json_typed!(borrowed, animations),
                },
                priority,
            },
        )
    };

    #[cfg(feature = "tracing")]
    tracing::debug!("FNIS Generated for animations: {json_path:?}: {patch:#?}");

    raw_borrowed_patches
        .0
        .entry(behavior_key)
        .or_default()
        .seq
        .insert(json_path, patch);
}
