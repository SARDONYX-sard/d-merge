mod add;
mod basic;
mod furniture;
mod offset_arm;

use std::borrow::Cow;
use std::path::Path;

use dashmap::DashSet;
use json_patch::{json_path, JsonPatch, JsonPath, Op, OpRangeKind, ValueWithPriority};
use rayon::prelude::*;
use simd_json::json_typed;
use snafu::ResultExt;
use winnow::Parser;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::list_parser::parse_fnis_list;
use crate::behaviors::tasks::fnis::patch_gen::add::generate_patch;
use crate::behaviors::tasks::patches::types::{
    BehaviorStringDataMap, BorrowedPatches, RawBorrowedPatches,
};
use crate::behaviors::tasks::templates::key::THIRD_PERSON_0_MASTER;
use crate::config::{ReportType, StatusReportCounter, StatusReporterFn};
use crate::errors::{Error, FailedParseFnisModListSnafu, Result};
use crate::results::filter_results;

/// For Seq patch
pub(crate) const PUSH_OP: OpRangeKind = OpRangeKind::Seq(json_patch::OpRange {
    op: Op::Add,
    range: 9998..9999,
});

/// Register a mod's root behavior (`behaviors\FNIS_<namespace>_Behavior.hkx`)
/// into `meshes\actors\character\behaviors\0_master.xml`.
///
/// - `behavior_id`: Unique identifier for the behavior (e.g., `#<namespace>${index}`).
/// - `behavior_path`: Path to the behavior file used in `hkbBehaviorReferenceGenerator.behavior_name`.
pub fn push_mod_root_behavior<'a>(
    owned_data: &'a OwnedFnisInjection,
) -> (
    (JsonPath<'a>, ValueWithPriority<'a>),
    (JsonPath<'a>, ValueWithPriority<'a>),
) {
    let namespace = owned_data.namespace.as_str();
    let priority = owned_data.priority;
    let behavior_path = owned_data.behavior_path.as_str();
    let new_root_behavior_index = owned_data.next_class_name_attribute();

    let one = (
        vec![
            Cow::Owned(new_root_behavior_index.clone()),
            Cow::Borrowed("hkbBehaviorReferenceGenerator"),
        ],
        ValueWithPriority {
            patch: JsonPatch {
                op: OpRangeKind::Pure(Op::Add),
                value: json_typed!(borrowed, {
                    "__ptr": new_root_behavior_index,
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

    let seq = (
        json_path!["#0340", "hkbStateMachine", "states"],
        ValueWithPriority {
            patch: JsonPatch {
                op: PUSH_OP,
                value: json_typed!(borrowed, new_root_behavior_index),
            },
            priority,
        },
    );

    (one, seq)
}

pub fn collect_borrowed_patches<'a>(
    owned_patches: &'a [OwnedFnisInjection],
    status_reporter: &'a StatusReporterFn,
) -> (BorrowedPatches<'a>, Vec<Error>) {
    let raw_borrowed_patches = RawBorrowedPatches::default();
    let template_keys = DashSet::new();
    let variable_class_map = BehaviorStringDataMap::new();

    let reporter = StatusReportCounter::new(
        status_reporter,
        ReportType::GeneratingFnisPatches,
        owned_patches.len(),
    );

    let results: Vec<Result<()>> = owned_patches
        .par_iter()
        .map(|owed_ref_one_mod| {
            reporter.increment();

            let priority = owed_ref_one_mod.priority;
            let (lists, errors): (Vec<_>, Vec<_>) = owed_ref_one_mod
                .list_contents
                .par_iter()
                .partition_map(|list| {
                    let result = parse_fnis_list
                        .parse(list)
                        .map_err(|e| serde_hkx::errors::readable::ReadableError::from_parse(e))
                        .with_context(|_| FailedParseFnisModListSnafu {
                            path: Path::new("TODO: Need list path").to_path_buf(),
                        });
                    match result {
                        Ok(list) => rayon::iter::Either::Left(list),
                        Err(err) => rayon::iter::Either::Right(err),
                    }
                });

            // TODO:
            // let (json_patches, _): (nemesis_xml::patch::PatchesMap, Vec<_>) = lists
            //     .into_par_iter()
            //     .flat_map(|list| generate_patch(owed_ref_one_mod, list))
            //     .collect();

            // template_keys.insert(key.clone());
            // if let Some(class_index) = variable_class_index {
            //     variable_class_map
            //         .0
            //         .entry(key.clone())
            //         .or_insert(class_index);
            // }

            // Mod Root register to actors/character/behaviors/0_master.xml
            {
                let entry = raw_borrowed_patches
                    .0
                    .entry(THIRD_PERSON_0_MASTER)
                    .or_default();
                let (one, seq) = push_mod_root_behavior(owed_ref_one_mod);
                entry.one.insert(one.0, one.1);
                entry.seq.insert(seq.0, seq.1);
            }

            // json_patches.into_par_iter().for_each(|(json_path, value)| {
            //     // FIXME: I think that if we lengthen the lock period, we can suppress the race condition, but that will slow down the process.
            //     let entry = raw_borrowed_patches.0.entry(key.clone()).or_default();

            //     match &value.op {
            //         // Overwrite to match patch structure
            //         // Pure: no add and remove because of single value
            //         json_patch::OpRangeKind::Pure(_) => {
            //             let value = ValueWithPriority::new(value, priority);
            //             entry.value().one.insert(json_path, value);
            //         }
            //         json_patch::OpRangeKind::Seq(_) => {
            //             let value = ValueWithPriority::new(value, priority);
            //             entry.value().seq.insert(json_path, value);
            //         }
            //         json_patch::OpRangeKind::Discrete(range_vec) => {} // never used. old API.
            //     }
            // });

            Ok(())
        })
        .collect();

    let errors = match filter_results(results) {
        Ok(()) => vec![],
        Err(errors) => errors,
    };

    (
        BorrowedPatches {
            template_keys,
            borrowed_patches: raw_borrowed_patches,
            behavior_string_data_map: variable_class_map,
        },
        errors,
    )
}
