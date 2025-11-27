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
//!
//! To
//!
//! ```txt
//! <skyrim data dir>/
//! └── meshes/
//!     └── actors/
//!         └── character/                                      <- defaultmale, defaultfemale humanoid animations
//!             └── animations/
//!                 └── OpenAnimationReplacer/
//!                     └── <fnis_mod_namespace>/               <- (Can rename FNIS_<namespace>_toOAR.json)
//!                         └── _1hm_eqp_0                      <- FNIS group name + Index. (Can rename FNIS_<namespace>_toOAR.json)
//!                              ├── 1hm_equip.hkx              <- HKX animation file.
//!                              ├── 1hm_unequip.HKX            <- HKX animation file.
//!                              └── config.json                <- OAR config file.
//! ```
mod generated_group_table;
mod oar_json;

use std::borrow::Cow;
use std::path::Path;
use std::path::PathBuf;

use crate::behaviors::tasks::fnis::patch_gen::alternative::oar_json::{
    prepare_anim_config_json, FnisToOarConfig,
};
use crate::behaviors::tasks::fnis::patch_gen::hkx_convert::{AnimIoJob, ConversionJob};
use crate::errors::Error;
use crate::{
    behaviors::tasks::fnis::{
        collect::owned::OwnedFnisInjection, list_parser::patterns::alt_anim::AlternativeAnimation,
    },
    Config,
};
use rayon::prelude::*;

/// Just write file
#[derive(Debug)]
pub struct AltAnimConfigJob {
    pub output_path: PathBuf,
    pub config: String,
}

pub fn alt_anim_to_oar(
    owned_data: &OwnedFnisInjection,
    alt_anim: AlternativeAnimation<'_>,
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
                        "Failed to parse FNIS alternative animation override config file '{}': {err}. Using default settings.",
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

        let Some(group) = generated_group_table::ALT_GROUPS.get(group_name) else {
            errors.push(Error::Custom {
                msg: "Not found alt groups table.".to_string(),
            });
            continue;
        };

        let group_jobs = (0..slots).into_par_iter().flat_map(|slot| {
            // each FNIS alt group output directory.(can rename by override config)
            let slot_config = group_config.and_then(|group_cfg| group_cfg.slots.get(&slot));

            let group_config_dir = slot_config
                .and_then(|slot| slot.rename_to.as_deref().map(Cow::Borrowed))
                .unwrap_or_else(|| Cow::Owned(format!("{group_name}_{slot}")));

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
                        need_copy: true,
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
