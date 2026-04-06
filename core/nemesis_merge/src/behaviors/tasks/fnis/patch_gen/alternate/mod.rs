//! # FNIS Alternate Animation to OAR
//!
//! Translates the FNIS alternate animation directory convention into the
//! [OpenAnimationReplacer](https://www.nexusmods.com/skyrimspecialedition/mods/92109)
//! (OAR) sub-directory format, and generates the supporting config files that
//! wire everything together at runtime.
//!
//! ## Requirements
//!
//! The output of this module requires the following Skyrim SE mods to be installed:
//!
//! - [OpenAnimationReplacer](https://www.nexusmods.com/skyrimspecialedition/mods/92109) — plays the correct animation slot based on OAR conditions
//! - [fnis_aa](https://github.com/SARDONYX-sard/fnis_aa) — overrides `FNIS_aa2` Papyrus natives to read slot layout from `config.json`
//!
//! ## Directory transformation XPMSSE(namespace = XPMSE)
//!
//! ### Before (FNIS layout)
//!
//! ```txt
//! Skyrim Special Edition/Data/
//! └── meshes/actors/character/animations/
//!     └── XPMSE/
//!         ├── FNIS_XPMSE_toOAR.json        <- optional rename/condition overrides
//!         ├── xpe0_1hm_equip.hkx           <- prefix + slot + animation name
//!         └── xpe0_1hm_unequip.hkx
//! ```
//!
//! ### After (OAR layout + generated config files)
//!
//! ```txt
//! Skyrim Special Edition/Data/
//! ├── meshes/actors/character/animations/
//! │   └── OpenAnimationReplacer/
//! │       └── XPMSE/
//! │           ├── config.json              <- namespace-level OAR priority config
//! │           └── xpe_1hmeqp_1/            <- group name + slot index(1..registered_slots_count)
//! │               ├── 1hm_equip.hkx        <- prefix and slot stripped from filename
//! │               ├── 1hm_unequip.hkx
//! │               └── config.json          <- slot-level OAR condition (FNISaa_1hmeqp == 1)
//! │
//! └── SKSE/Plugins/
//!     └── fnis_aa/
//!         └── config.json                  <- mod/group/base slot layout consumed by fnis_aa.dll at runtime
//! ```
//!
//! ## Runtime flow
//!
//! ```txt
//! XPMSE MCM sets a style for an actor
//!         ↓
//! FNIS_aa.SetAnimGroupEX(actor, "_1hmeqp", base, styleIndex)
//!         ↓  intercepted by fnis_aa.dll, reads base from fnis_aa/config.json
//! actor.SetAnimationVariableInt("FNISaa_1hmeqp", base + styleIndex)
//!         ↓
//! OAR evaluates the condition in each slot directory:
//!   xpe_1hmeqp_1/config.json  ->  FNISaa_1hmeqp == 1  ->  skip
//!   xpe_1hmeqp_2/config.json  ->  FNISaa_1hmeqp == 2  ->  match -> plays Back style
//! ```
pub(crate) mod aa_config;
pub(crate) mod generated_group_table;
pub(crate) mod group_names;
pub(crate) mod oar_json;
pub(crate) mod override_config;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use fnis_list::patterns::alt_anim::AlternateAnimation;
use rayon::prelude::*;

use self::{
    group_names::AAGroupName,
    override_config::{FnisToOarConfig, SlotConfig},
};
use crate::{
    behaviors::tasks::fnis::{
        collect::owned::OwnedFnisInjection,
        patch_gen::io_jobs::{AnimIoJob, AnimKind, ConversionJob},
    },
    errors::Error,
    Config,
};

/// Just write file
#[derive(Debug)]
pub struct FnisAANamespaceConfigJob {
    pub output_path: PathBuf,
    pub config: String,
}

/// Deferred slot-level config job; `base` is resolved at write time from the
/// computed `AAConfig` base map.
///
/// e.g., `_1hmeqp_1`
#[derive(Debug)]
pub struct FnisAASlotConfigJob {
    pub output_path: PathBuf,
    /// The FNIS group enum, used to look up the computed base.
    pub group_name: AAGroupName,
    /// Mod prefix, used as the key to look up `mod_id` / `base` in `AAConfig`.
    pub prefix: Arc<str>,
    /// 0-based slot index within this mod's group.
    pub slot: u64,

    /// Optional user-defined per-slot overrides (rename, priority, conditions).
    pub slot_config: Option<Arc<SlotConfig>>, // owned copy
    /// The directory name shown in OAR GUI (already resolved by caller).
    pub group_config_dir: String,
}

pub fn alt_anim_to_oar<'a>(
    owned_data: &'a OwnedFnisInjection,
    alt_anim: AlternateAnimation<'a>,
    config: &Config,
) -> (Vec<AnimIoJob>, Vec<Error>) {
    let namespace = &owned_data.namespace;
    // NOTE: The lifetime is not an issue.
    // Arc<str> clones a byte array within the heap and reuses it.
    let prefix = Arc::from(alt_anim.prefix);

    let mut errors = vec![];

    let override_config = owned_data.alt_anim_config.as_deref().and_then(|data|
        match sonic_rs::from_slice::<FnisToOarConfig>(data) {
            Ok(cfg) => Some(cfg),
            Err(err) => {
                let override_config_path = owned_data.to_fnis_aa_override_config_path();
                errors.push(Error::Custom {
                    msg: format!(
                        "Failed to parse FNIS alternate animation override config file '{}': {err}. Using default settings.",
                        override_config_path.display(),
                    ),
                });
                None
            }
        }
    );
    #[cfg(feature = "tracing")]
    tracing::debug!("Using FNIS to OAR override config: {override_config:#?}");

    let output_dir = {
        let base_dir = owned_data.behavior_entry.base_dir;
        let output_dir = &config.output_dir;

        let mut output_dir = output_dir.clone();
        output_dir.push("meshes");
        output_dir.push(base_dir);
        output_dir.push("animations");
        output_dir.push("OpenAnimationReplacer");
        output_dir.push(
            override_config
                .as_ref()
                .and_then(|c| c.name.as_deref())
                .unwrap_or(namespace),
        );
        output_dir
    };

    let mut ret_jobs = vec![];
    ret_jobs.push(AnimIoJob::FnisAANamespaceConfig(FnisAANamespaceConfigJob {
        output_path: output_dir.join("config.json"),
        config: oar_json::prepare_namespace_json(namespace, &override_config),
    }));

    for set in &alt_anim.set {
        let registered_slots_count = set.slots;
        let group_name = set.group;

        let group_config = override_config
            .as_ref()
            .and_then(|c| c.groups.get(group_name));

        // Validate group
        let Ok(group_aa_name) = group_name.parse::<AAGroupName>() else {
            errors.push(Error::Custom {
                msg: format!("Unknown FNIS AA group name: {group_name}"),
            });
            continue;
        };

        let Some(group) = generated_group_table::ALT_GROUPS.get(group_aa_name.as_fnis_str()) else {
            // Should be unreachable.
            errors.push(Error::Custom {
                msg: format!("Not found alt groups table. {group_name}"),
            });
            continue;
        };

        let group_jobs = (0..registered_slots_count)
            .into_par_iter()
            .flat_map(|slot| {
                // each FNIS alt group output directory.(can rename by override config)
                let slot_config =
                    group_config.and_then(|group_cfg| group_cfg.slots.get(&slot).map(Arc::clone));

                // NOTE: group_config_dir will ultimately own the directory. It’s fine to clone it here.
                let group_config_dir =
                    match slot_config.as_deref().and_then(|s| s.rename_to.as_deref()) {
                        Some(name) => name.to_string(),
                        // Include the prefix. Otherwise, if two `group_name`s are declared, they will conflict.
                        None => format!("{prefix}{group_name}_{}", slot + 1), // Honestly, there’s no need to start from 1 (for compatibility reasons, I’ll keep the rules I established in the past).
                    };

                let slot_output_dir = output_dir.join(&group_config_dir);

                // hkx jobs
                let mut jobs: Vec<_> = group
                    .animations
                    .par_iter()
                    .map(|animation| {
                        let (input_path, output_inner, is_male_subdir) =
                            resolve_animation_path(owned_data, &prefix, slot, animation);

                        AnimIoJob::Hkx(ConversionJob {
                            input_path,
                            output_path: slot_output_dir.join(output_inner),
                            kind: AnimKind::FnisAA {
                                prefix: Arc::clone(&prefix),
                                group_name: group_aa_name,
                                slot_count: registered_slots_count,
                                is_male_subdir,
                            },
                        })
                    })
                    .collect();

                // config job
                jobs.push(AnimIoJob::FnisAASlotConfig(FnisAASlotConfigJob {
                    output_path: slot_output_dir.join("config.json"),
                    group_name: group_aa_name,
                    prefix: Arc::clone(&prefix),
                    slot,
                    slot_config,
                    group_config_dir,
                }));

                jobs
            });

        ret_jobs.par_extend(group_jobs);
    }

    (ret_jobs, errors)
}

/// Returns `(input_path, output_relative_path, is_male_subdir)`.
///
/// When the source animation lives under a `male/` subdirectory, a mirrored
/// `female/` copy is also emitted so both genders play the replacement.
fn resolve_animation_path(
    owned_data: &OwnedFnisInjection,
    prefix: &str,
    slot: u64,
    animation: &str,
) -> (PathBuf, PathBuf, bool) {
    let animation_path = Path::new(animation);

    // NOTE: Based on what I can see in FNISSexyMove,
    // male/mt_runforward.hkx was listed as namespace/DF1_mt_runforward.hkx.
    // "dlc01" or "male"
    let Some(file_name) = animation_path.file_name().and_then(|s| s.to_str()) else {
        return (
            owned_data
                .animations_mod_dir
                .join(format!("{prefix}{slot}_{animation}")),
            animation_path.to_path_buf(),
            false,
        );
    };

    let parent = animation_path.parent().unwrap_or_else(|| Path::new(""));
    let is_male_subdir = parent
        .file_name()
        .is_some_and(|s| s.eq_ignore_ascii_case("male"));

    if is_male_subdir {
        (
            owned_data
                .animations_mod_dir
                .join(format!("{prefix}{slot}_{file_name}")),
            animation_path.to_path_buf(), // `male/` from output
            true,
        )
    } else if parent.as_os_str().is_empty() {
        (
            owned_data
                .animations_mod_dir
                .join(format!("{prefix}{slot}_{file_name}")),
            PathBuf::from(file_name),
            false,
        )
    } else {
        (
            owned_data
                .animations_mod_dir
                .join(parent)
                .join(format!("{prefix}{slot}_{file_name}")),
            animation_path.to_path_buf(),
            false,
        )
    }
}
