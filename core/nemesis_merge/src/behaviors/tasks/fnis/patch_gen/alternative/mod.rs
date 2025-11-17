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
use std::{fs, path::PathBuf};

use crate::behaviors::tasks::fnis::patch_gen::alternative::oar_json::{
    write_anim_config_json, FnisToOarConfig,
};
use crate::behaviors::tasks::fnis::patch_gen::hkx_convert::{check_hkx_header, convert_hkx};
use crate::errors::Error;
use crate::OutPutTarget;
use crate::{
    behaviors::tasks::fnis::{
        collect::owned::OwnedFnisInjection, list_parser::patterns::alt_anim::AlternativeAnimation,
    },
    Config,
};
use rayon::iter::Either;
use rayon::prelude::*;

pub fn alt_anim_to_oar(
    owned_data: &OwnedFnisInjection,
    alt_anim: AlternativeAnimation<'_>,
    config: &Config,
) -> Result<(), Vec<Error>> {
    let base_dir = owned_data.behavior_entry.base_dir;
    let namespace = &owned_data.namespace;
    let output_dir = &config.output_dir;
    let output_format = config.output_target;
    let prefix = alt_anim.prefix;

    let mut errors = Vec::new();

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
    let mut owned_override_config = std::fs::read(&override_config_path).ok();
    let override_config: FnisToOarConfig = match owned_override_config.as_deref_mut() {
        Some(data) => match simd_json::from_slice::<FnisToOarConfig>(data) {
            Ok(cfg) => cfg,
            Err(err) => {
                errors.push(Error::Custom {
                    msg: format!(
                        "Failed to parse FNIS alternative animation override config file '{}': {}. Using default settings.",
                        override_config_path.display(),
                        err
                    ),
                });
                FnisToOarConfig::default()
            }
        },
        None => FnisToOarConfig::default(),
    };

    let output_dir = {
        let mut output_dir = output_dir.clone();
        output_dir.push("meshes");
        output_dir.push(base_dir);
        output_dir.push("OpenAnimationReplacer");
        output_dir.push(override_config.name.as_deref().unwrap_or(namespace));
        output_dir
    };

    if let Err(err) = oar_json::write_namespace_json(namespace, &output_dir, &override_config) {
        errors.push(err);
    }

    for set in &alt_anim.set {
        let slots = set.slots;
        let group_name = set.group;

        let Some(group) = generated_group_table::ALT_GROUPS.get(group_name) else {
            errors.push(Error::Custom {
                msg: "Not found alt groups table.".to_string(),
            });
            continue;
        };

        errors.par_extend((0..slots).into_par_iter().flat_map(|slot| {
            // each FNIS alt group output directory.(can rename by override config)
            let group_config_dir = override_config
                .groups
                .get(group_name)
                .and_then(|group_cfg| group_cfg.slots.get(&slot))
                .and_then(|slot| slot.rename_to.as_deref().map(Cow::Borrowed))
                .unwrap_or_else(|| Cow::Owned(format!("{group_name}_{slot}")));

            let output_dir = output_dir.join(group_config_dir.as_ref());

            let (_, mut errs): ((), Vec<Error>) =
                group.animations.par_iter().partition_map(|animation| {
                    let animation_path = Path::new(animation);
                    let input = if let (Some(parent), Some(file_name)) = (
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

                    if let Err(e) = process_hkx(&input, &output_dir, animation, output_format) {
                        return Either::Right(e);
                    };

                    Either::Left(())
                });

            if let Err(e) = write_anim_config_json(
                &output_dir,
                &group_config_dir,
                group_name,
                slot,
                &override_config,
            ) {
                errs.push(e);
            };

            errs
        }));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(())
}

/// This is necessary because Unix systems are case-sensitive.
fn find_case_insensitive(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let file_name = path.file_name()?;

    for entry in fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        if name.eq_ignore_ascii_case(file_name) {
            return Some(entry.path());
        }
    }
    None
}

fn process_hkx(
    input: &Path,
    output_dir: &Path,
    animation: &str,
    output_format: OutPutTarget,
) -> Result<(), Error> {
    // FIXME: Exists sometimes misjudges virtualization as unstable for some reason in MO2.
    let actual_input: Cow<Path> = if input.exists() {
        Cow::Borrowed(input)
    } else if let Some(found) = find_case_insensitive(input) {
        Cow::Owned(found)
    } else {
        #[cfg(feature = "tracing")]
        tracing::info!(
            "FNIS alternative animation input file '{}' not found. Then Skipped.",
            input.display()
        );
        return Ok(());
    };

    let current_format = check_hkx_header(input, output_format)?;
    let output = output_dir.join(animation);

    if current_format != output_format {
        convert_hkx(&actual_input, &output, output_format)?;
    } else {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|err| Error::FailedIo {
                path: parent.to_path_buf(),
                source: err,
            })?;
        }

        fs::copy(&actual_input, &output).map_err(|err| Error::FailedIo {
            path: output,
            source: err,
        })?;
    }

    Ok(())
}
