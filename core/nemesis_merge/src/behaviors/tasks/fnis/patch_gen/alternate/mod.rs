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
//! - [BehaviorDataInjector](https://www.nexusmods.com/skyrimspecialedition/mods/78146) — injects `FNISaa_*` variables into the Havok behavior graph
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
//! │           └── _1hmeqp_1/               <- group name + slot index(1 based. 0 is vanilla)
//! │               ├── 1hm_equip.hkx        <- prefix and slot stripped from filename
//! │               ├── 1hm_unequip.hkx
//! │               └── config.json          <- slot-level OAR condition (FNISaa_1hmeqp == 1)
//! │
//! └── SKSE/Plugins/
//!     ├── BehaviorDataInjector/
//!     │   └── FNIS_AA_to_OAR_BDI.json      <- declares FNISaa_* kInt variables in
//!     │                                        the Havok behavior graph
//!     └── fnis_aa/
//!         └── config.json                  <- mod/group/base slot layout consumed
//!                                             by fnis_aa.dll at runtime
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
//!   _1hmeqp_1/config.json  ->  FNISaa_1hmeqp == 1  ->  skip
//!   _1hmeqp_2/config.json  ->  FNISaa_1hmeqp == 2  ->  match -> plays Back style
//! ```
//!
//! Havok Behavior graph variables are stored per-actor instance, so Player and
//! NPCs each carry their own value for `FNISaa_<group>`. This means the MCM can
//! assign different styles to the player and to NPCs independently, even though
//! both read from the same variable name.
//!
//! ## Entry points
//!
//! - [`alt_anim_to_oar`] — processes one [`OwnedFnisInjection`] and returns the
//!   full list of [`AnimIoJob`]s (HKX copies + config writes) for that mod.
//! - [`bdi::generate_bdi_config`] — writes `FNIS_AA_to_OAR_BDI.json` once for
//!   the entire session.
//! - [`aa_config::generate_aa_config_from_jobs`] — call once after all
//!   [`alt_anim_to_oar`] jobs have been collected to write `fnis_aa/config.json`.
pub(crate) mod aa_config;
pub(crate) mod bdi;
mod generated_group_table;
pub(crate) mod group_names;
mod oar_json;

use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;

use fnis_list::patterns::alt_anim::AlternateAnimation;
use rayon::prelude::*;

use crate::behaviors::tasks::fnis::collect::owned::OwnedFnisInjection;
use crate::behaviors::tasks::fnis::patch_gen::{
    alternate::{
        group_names::AAGroupName,
        oar_json::{prepare_anim_config_json, FnisToOarConfig},
    },
    hkx_convert::{AnimIoJob, AnimKind, ConversionJob},
};
use crate::errors::Error;
use crate::Config;

/// Just write file
#[derive(Debug)]
pub struct AltAnimConfigJob {
    pub output_path: PathBuf,
    pub config: String,
}

pub fn alt_anim_to_oar(
    owned_data: &OwnedFnisInjection,
    alt_anim: AlternateAnimation<'_>,
    config: &Config,
) -> (Vec<AnimIoJob>, Vec<Error>) {
    let namespace = &owned_data.namespace;
    let prefix = alt_anim.prefix;

    let mut errors = vec![];

    let override_config: FnisToOarConfig = match owned_data.alt_anim_config.as_deref() {
        Some(data) => match sonic_rs::from_slice::<FnisToOarConfig>(data) {
            Ok(cfg) => cfg,
            Err(err) => {
                let override_config_path = {
                    let filename = if owned_data.behavior_entry.is_humanoid() {
                        format!("FNIS_{namespace}_toOAR.json")
                    } else {
                        format!(
                            "FNIS_{namespace}_{}_toOAR.json",
                            owned_data.behavior_entry.behavior_object // e.g, "dog", "horse"
                        )
                    };
                    owned_data.animations_mod_dir.join(filename)
                };
                errors.push(Error::Custom {
                    msg: format!(
                        "Failed to parse FNIS alternate animation override config file '{}': {err}. Using default settings.",
                        override_config_path.display(),
                    ),
                });
                FnisToOarConfig::default()
            }
        },
        None => FnisToOarConfig::default(),
    };
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
        output_dir.push(override_config.name.as_deref().unwrap_or(namespace));
        output_dir
    };

    let mut ret_jobs = vec![];
    let config_job = AltAnimConfigJob {
        output_path: output_dir.join("config.json"),
        config: oar_json::prepare_namespace_json(namespace, &override_config),
    };
    ret_jobs.push(AnimIoJob::Config(config_job));

    for set in &alt_anim.set {
        let slots = set.slots;
        let group_name = set.group;
        let group_config = override_config.groups.get(group_name);

        // Validate group name early; unknown names are skipped rather than
        // propagating a String that silently fails later in aa_config.
        let Ok(group_aa_name) = group_name.parse::<AAGroupName>() else {
            errors.push(Error::Custom {
                msg: format!("Unknown FNIS AA group name: {group_name}"),
            });
            continue;
        };

        let Some(group) = generated_group_table::ALT_GROUPS.get(group_name) else {
            errors.push(Error::Custom {
                msg: format!("Not found alt groups table. {group_name}"),
            });
            continue;
        };

        let group_jobs = (0..slots).into_par_iter().flat_map(|slot| {
            // each FNIS alt group output directory.(can rename by override config)
            let slot_config = group_config.and_then(|group_cfg| group_cfg.slots.get(&slot));

            let group_config_dir = slot_config
                .and_then(|slot| slot.rename_to.as_deref().map(Cow::Borrowed))
                .unwrap_or_else(|| Cow::Owned(format!("{group_name}_{}", slot + 1))); // Make the OAR GUI appearance consistent with 1based as well

            let output_dir = output_dir.join(group_config_dir.as_ref());

            let mut jobs: Vec<_> = group
                .animations
                .par_iter()
                .map(|animation| {
                    let animation_path = Path::new(animation);
                    let input_path = if let (Some(parent), Some(file_name)) = (
                        animation_path.parent(),
                        animation_path.file_name().and_then(|s| s.to_str()),
                    ) {
                        owned_data
                            .animations_mod_dir
                            .join(parent)
                            .join(format!("{prefix}{slot}_{file_name}"))
                    } else {
                        owned_data
                            .animations_mod_dir
                            .join(format!("{prefix}{slot}_{animation}"))
                    };

                    AnimIoJob::Hkx(ConversionJob {
                        input_path,
                        output_path: output_dir.join(animation_path),
                        kind: AnimKind::FnisAA {
                            prefix: prefix.to_string(),
                            group_name: group_aa_name,
                            slot_count: slots,
                        },
                    })
                })
                .collect();

            jobs.push(AnimIoJob::Config(AltAnimConfigJob {
                output_path: output_dir.join("config.json"),
                config: prepare_anim_config_json(&group_config_dir, group_name, slot, slot_config),
            }));

            jobs
        });

        ret_jobs.par_extend(group_jobs);
    }

    (ret_jobs, errors)
}
