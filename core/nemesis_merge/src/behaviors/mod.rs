//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
mod priority_ids;
pub(crate) mod tasks;

pub use crate::behaviors::priority_ids::types::{PatchMaps, PriorityMap};
use crate::behaviors::tasks::fnis::collect::collect_all_fnis_injections;
pub use tasks::templates::gen_bin::create_bin_templates;

pub(crate) use tasks::{
    adsf::path_parser::ParseError as AsdfPathParseError,
    asdsf::path_parser::ParseError as AsdsfPathParseError,
};

use crate::behaviors::tasks::adsf::apply_adsf_patches;
use crate::behaviors::tasks::asdsf::apply_asdsf_patches;
use crate::behaviors::tasks::hkx::generate::generate_hkx_files;
use crate::behaviors::tasks::patches::types::OwnedPatches;
use crate::behaviors::tasks::patches::{
    apply::apply_patches,
    collect::{collect_borrowed_patches, collect_owned_patches},
    types::{BorrowedPatches, OwnedPatchMap},
};
use crate::config::{Config, Status};
use crate::errors::{writer::write_errors, BehaviorGenerationError, Error, Result};
use rayon::prelude::*;

/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(patches: PatchMaps, config: Config) -> Result<()> {
    let PatchMaps {
        nemesis_entries,
        fnis_entries,
    } = patches;

    #[cfg(feature = "tracing")]
    {
        let mut sorted: Vec<_> = nemesis_entries.par_iter().collect();
        sorted.par_sort_by_key(|&(_, v)| *v);
        tracing::debug!("nemesis_entries = {sorted:#?}");

        let mut sorted: Vec<_> = fnis_entries.par_iter().collect();
        sorted.par_sort_by_key(|&(_, v)| *v);
        tracing::debug!("fnis_entries = {sorted:#?}");
    }

    let (owned_fnis_patches, mut fnis_errors) = if !fnis_entries.is_empty() {
        let skyrim_data_dir_glob = config
            .skyrim_data_dir_glob
            .as_ref()
            .ok_or(Error::MissingSkyrimDataDirGlob)?;
        collect_all_fnis_injections(skyrim_data_dir_glob, &fnis_entries)
    } else {
        (vec![], vec![])
    };

    let (fnis_hkx_patches, fnis_adsf_patches) = {
        let (fnis_hkx_patches, fnis_adsf_patches, errors) =
            tasks::fnis::patch_gen::collect_borrowed_patches(
                &owned_fnis_patches,
                &config.status_report,
            );
        fnis_errors.par_extend(errors);

        (fnis_hkx_patches, fnis_adsf_patches)
    };

    // Collect all patches file.
    let OwnedPatches {
        owned_patches,
        adsf_patches: owned_adsf_patches,
        asdsf_patches: owned_asdsf_patches,
        errors: owned_file_errors,
    } = collect_owned_patches(&nemesis_entries, &config).await;

    let mut adsf_errors = vec![];
    let mut asdsf_errors = vec![];
    let mut patched_hkx_errors = None;

    rayon::scope(|s| {
        s.spawn(|_| {
            adsf_errors = apply_adsf_patches(
                owned_adsf_patches,
                &nemesis_entries,
                &config,
                fnis_adsf_patches,
            );
        });
        s.spawn(|_| {
            asdsf_errors = apply_asdsf_patches(owned_asdsf_patches, &nemesis_entries, &config);
        });
        s.spawn(|_| {
            patched_hkx_errors = Some(apply_and_gen_patched_hkx(
                &owned_patches,
                &config,
                fnis_hkx_patches,
            ));
        });
    });

    // Error process
    {
        let Errors {
            patch_errors_len,
            apply_errors_len,
            hkx_errors_len,
            hkx_errors,
        } = patched_hkx_errors.unwrap_or_default();
        let fnis_errors_errors_len = fnis_errors.len();
        let owned_file_errors_len = owned_file_errors.len();
        let adsf_errors_len = adsf_errors.len();
        let asdsf_errors_len = asdsf_errors.len();

        let all_errors = {
            let mut all_errors = vec![];
            all_errors.par_extend(fnis_errors);
            all_errors.par_extend(owned_file_errors);
            all_errors.par_extend(adsf_errors);
            all_errors.par_extend(asdsf_errors);
            all_errors.par_extend(hkx_errors);
            all_errors
        };

        if !all_errors.is_empty() {
            let err = BehaviorGenerationError {
                fnis_errors_errors_len,
                owned_file_errors_len,
                adsf_errors_len,
                asdsf_errors_len,
                patch_errors_len,
                apply_errors_len,
                hkx_errors_len,
            };
            config.on_report_status(Status::Error(err.to_string()));

            write_errors(&config, &all_errors).await?;
            return Err(Error::FailedToGenerateBehaviors { source: err });
        };
    }

    config.on_report_status(Status::Done);
    Ok(())
}

#[derive(Default)]
struct Errors {
    patch_errors_len: usize,
    apply_errors_len: usize,
    hkx_errors_len: usize,
    hkx_errors: Vec<Error>,
}

fn apply_and_gen_patched_hkx<'a>(
    owned_patches: &'a OwnedPatchMap,
    config: &Config,
    fnis_patches: BorrowedPatches<'a>,
) -> Errors {
    let mut all_errors = vec![];

    // 1/3: Parse nemesis patches
    let (
        BorrowedPatches {
            template_keys: template_names,
            borrowed_patches,
            behavior_string_data_map: variable_class_map,
        },
        patch_errors_len,
    ) = {
        let (borrowed_patches, errors) =
            collect_borrowed_patches(owned_patches, config, fnis_patches);

        let patch_errors_len = errors.len();
        all_errors.par_extend(errors);

        // borrowed_patches
        (borrowed_patches, patch_errors_len)
    };
    #[cfg(feature = "tracing")]
    tracing::debug!("needed template_names = {template_names:#?}");

    let mut template_error_len;
    let owned_templates = {
        use self::tasks::templates::collect::owned;
        let template_dir = &config.resource_dir;
        // NOTE: Since `DashSet` cannot solve the lifetime error of `contain`, we have no choice but to replace it with `HashSet`.
        let (owned_templates, errors) =
            owned::collect_templates(template_dir, template_names.into_par_iter().collect());
        template_error_len = errors.len();
        all_errors.par_extend(errors);
        owned_templates
    };

    {
        // NOTE: Without this seemingly meaningless move, an lifetime error is made.
        // Need `'owned_templates`: `'variable_class_map` & `'borrowed_patches`. So let's move here and shrink these lifetimes.
        let variable_class_map = variable_class_map;
        let borrowed_patches = borrowed_patches;

        let mut templates = {
            use self::tasks::templates::collect::borrowed;
            let (templates, errors) = borrowed::collect_templates(&owned_templates);
            template_error_len += errors.len();
            all_errors.par_extend(errors);
            templates
        };

        #[cfg(feature = "tracing")]
        {
            tracing::debug!("owned_templates_keys = {:#?}", owned_templates.keys());
            tracing::debug!("borrowed_templates_keys = {:#?}", {
                let borrowed_keys: Vec<String> =
                    templates.par_iter().map(|r| r.key().to_string()).collect();
                borrowed_keys
            });
        }

        // 2/3: Apply patches & Replace variables to indexes
        let mut apply_errors_len = template_error_len;
        if let Err(errors) = apply_patches(&mut templates, borrowed_patches, config) {
            apply_errors_len = errors.len();
            all_errors.par_extend(errors);
        };

        // 3/3: Generate hkx files.
        let hkx_errors_len = {
            if let Err(hkx_errors) = generate_hkx_files(config, templates, variable_class_map) {
                let errors_len = hkx_errors.len();
                all_errors.par_extend(hkx_errors);
                errors_len
            } else {
                0
            }
        };

        Errors {
            patch_errors_len,
            apply_errors_len,
            hkx_errors_len,
            hkx_errors: all_errors,
        }
    }
}
