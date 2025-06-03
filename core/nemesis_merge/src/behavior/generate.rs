//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    config::{Config, Status},
    errors::{write_errors::write_errors, BehaviorGenerationError, Error, Result},
    hkx::generate::generate_hkx_files,
    patches::{
        apply::apply_patches,
        collect::{collect_borrowed_patches, collect_owned_patches, BorrowedPatches},
    },
    path_id::paths_to_priority_map,
    types::OwnedPatchMap,
};
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
    config.on_report_status(Status::ReadingTemplatesAndPatches);
    let (owned_adsf_patches, owned_patches, owned_file_errors) =
        collect_owned_patches(&nemesis_paths, &id_order).await;

    // - Patch to `animationdatasinglefile.txt`
    // - Patch to hkx( -> xml)
    let (adsf_errors, patched_hkx_errors) = rayon::join(
        || crate::adsf::apply_adsf_patches(owned_adsf_patches, &id_order, &config),
        || apply_and_gen_patched_hkx(&owned_patches, &config),
    );

    let Errors {
        patch_errors_len,
        apply_errors_len,
        hkx_errors_len,
        hkx_errors,
    } = patched_hkx_errors;
    let owned_file_errors_len = owned_file_errors.len();
    let adsf_errors_len = adsf_errors.len();

    let all_errors = {
        let mut all_errors = vec![];
        all_errors.par_extend(owned_file_errors);
        all_errors.par_extend(adsf_errors);
        all_errors.par_extend(hkx_errors);
        all_errors
    };

    if !all_errors.is_empty() {
        let err = BehaviorGenerationError {
            owned_file_errors_len,
            adsf_errors_len,
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

struct Errors {
    patch_errors_len: usize,
    apply_errors_len: usize,
    hkx_errors_len: usize,
    hkx_errors: Vec<Error>,
}

fn apply_and_gen_patched_hkx(owned_patches: &OwnedPatchMap, config: &Config) -> Errors {
    let mut all_errors = vec![];

    // 1/2: Apply patches & Replace variables to indexes
    config.on_report_status(Status::ApplyingPatches);
    let (
        BorrowedPatches {
            template_names,
            patch_map_foreach_template,
            variable_class_map,
        },
        patch_errors_len,
    ) = {
        let (patch_result, errors) = collect_borrowed_patches(owned_patches, config.hack_options);
        let patch_errors_len = errors.len();
        all_errors.par_extend(errors);
        (patch_result, patch_errors_len)
    };
    #[cfg(feature = "tracing")]
    tracing::debug!("needed template_names = {template_names:#?}");

    let owned_templates = {
        use crate::templates::collect::owned;
        let template_dir = &config.resource_dir;
        owned::collect_templates(template_dir, template_names)
    };

    let template_error_len;
    let templates = {
        use crate::templates::collect::borrowed;
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
                templates.par_iter().map(|r| r.key().clone()).collect();
            borrowed_keys
        });
    }

    let mut apply_errors_len = template_error_len;
    if let Err(errors) = apply_patches(&templates, patch_map_foreach_template, &config.output_dir) {
        apply_errors_len = errors.len();
        all_errors.par_extend(errors);
    };

    // 2/2: Generate hkx files.
    config.on_report_status(Status::GenerateHkxFiles);
    let hkx_errors_len = {
        if let Err(hkx_errors) =
            generate_hkx_files(&config.output_dir, templates, variable_class_map)
        {
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
