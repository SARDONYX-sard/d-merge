//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
mod priority_ids;
mod tasks;

pub(crate) use tasks::adsf::path_parser::ParseError as AsdfPathParseError;
pub(crate) use tasks::asdsf::path_parser::ParseError as AsdsfPathParseError;
pub use tasks::templates::{gen_bin::create_bin_templates, TemplateError};

use self::priority_ids::paths_to_priority_map;
use self::tasks::{
    hkx::generate::generate_hkx_files,
    patches::{
        apply::apply_patches,
        collect::{collect_borrowed_patches, collect_owned_patches},
        types::{BorrowedPatches, OwnedPatchMap},
    },
};
use crate::behaviors::tasks::adsf::apply_adsf_patches;
use crate::behaviors::tasks::asdsf::apply_asdsf_patches;
use crate::behaviors::tasks::patches::types::OwnedPatches;
use crate::config::{Config, Status};
use crate::errors::{writer::write_errors, BehaviorGenerationError, Error, Result};
use rayon::prelude::*;
use std::path::PathBuf;

/// - nemesis_paths: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(nemesis_paths: Vec<PathBuf>, config: Config) -> Result<()> {
    let id_order = paths_to_priority_map(&nemesis_paths);
    #[cfg(feature = "tracing")]
    {
        let mut sorted: Vec<_> = id_order.par_iter().collect();
        sorted.par_sort_by_key(|&(_, v)| *v);
        tracing::trace!("id_order_by_priority = {sorted:#?}");
    }

    // Collect all patches file.
    let OwnedPatches {
        owned_patches,
        adsf_patches: owned_adsf_patches,
        asdsf_patches: owned_asdsf_patches,
        errors: owned_file_errors,
    } = collect_owned_patches(&nemesis_paths, &id_order, &config).await;

    let mut adsf_errors = vec![];
    let mut asdsf_errors = vec![];
    let mut patched_hkx_errors = None;

    rayon::scope(|s| {
        s.spawn(|_| adsf_errors = apply_adsf_patches(owned_adsf_patches, &id_order, &config));
        s.spawn(|_| asdsf_errors = apply_asdsf_patches(owned_asdsf_patches, &id_order, &config));
        s.spawn(|_| patched_hkx_errors = Some(apply_and_gen_patched_hkx(&owned_patches, &config)));
    });

    let Errors {
        patch_errors_len,
        apply_errors_len,
        hkx_errors_len,
        hkx_errors,
    } = patched_hkx_errors.unwrap_or_default();
    let owned_file_errors_len = owned_file_errors.len();
    let adsf_errors_len = adsf_errors.len();
    let asdsf_errors_len = asdsf_errors.len();

    let all_errors = {
        let mut all_errors = vec![];
        all_errors.par_extend(owned_file_errors);
        all_errors.par_extend(adsf_errors);
        all_errors.par_extend(asdsf_errors);
        all_errors.par_extend(hkx_errors);
        all_errors
    };

    if !all_errors.is_empty() {
        let err = BehaviorGenerationError {
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

fn apply_and_gen_patched_hkx(owned_patches: &OwnedPatchMap, config: &Config) -> Errors {
    let mut all_errors = vec![];

    // 1/3: Parse nemesis patches
    let (
        BorrowedPatches {
            template_names,
            borrowed_patches,
            behavior_string_data_map: variable_class_map,
        },
        patch_errors_len,
    ) = {
        let (patch_result, errors) = collect_borrowed_patches(owned_patches, config);
        let patch_errors_len = errors.len();
        all_errors.par_extend(errors);
        (patch_result, patch_errors_len)
    };
    #[cfg(feature = "tracing")]
    tracing::debug!("needed template_names = {template_names:#?}");

    let owned_templates = {
        use self::tasks::templates::collect::owned;
        let template_dir = &config.resource_dir;
        // NOTE: Since `DashSet` cannot solve the lifetime error of `contain`, we have no choice but to replace it with `HashSet`.
        owned::collect_templates(template_dir, template_names.into_par_iter().collect())
    };

    {
        // NOTE: Without this seemingly meaningless move, an lifetime error is made.
        // Need `'owned_templates`: `'variable_class_map` & `'borrowed_patches`. So let's move here and shrink these lifetimes.
        let variable_class_map = variable_class_map;
        let borrowed_patches = borrowed_patches;

        let template_error_len;
        let mut templates = {
            use self::tasks::templates::collect::borrowed;
            let (templates, errors) = borrowed::collect_templates(&owned_templates);
            template_error_len = errors.len();
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
