//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    config::{Config, Status},
    errors::{write_errors::write_errors, Error, Result},
    hkx::generate::generate_hkx_files,
    patches::{
        apply::apply_patches,
        collect::{collect_borrowed_patches, collect_owned_patches, PatchResult},
        merge::{merge_patches, paths_to_ids},
    },
    templates::collect::collect_templates,
};
use rayon::prelude::*;
use std::path::PathBuf;

/// - nemesis_paths: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(nemesis_paths: Vec<PathBuf>, options: Config) -> Result<()> {
    let error_output = options.output_dir.join("d_merge_errors.log");
    let mut all_errors = vec![];

    // 1/4:
    let owned_patches = match collect_owned_patches(&nemesis_paths) {
        Ok(owned_patches) => owned_patches,
        Err(errors) => {
            let errors_len = errors.len();
            write_errors(&error_output, &errors).await?;
            return Err(Error::FailedToReadOwnedPatches { errors_len });
        }
    };

    options.report_status(Status::ReadingTemplatesAndPatches);
    let (patch_result, errors) = collect_borrowed_patches(&owned_patches);
    let PatchResult {
        template_names,
        template_patch_map,
        ptr_map,
    } = patch_result;
    let patch_errors_len = errors.len();
    all_errors.par_extend(errors);

    let ids = paths_to_ids(&nemesis_paths);
    {
        // HACK: Lifetime inversion hack: `templates` require `patch_mod_map` to live longer than `templates`, but `templates` actually live longer than `templates`.
        // Therefore, reassign the local variable in the block to shorten the lifetime
        let (templates, errors) = collect_templates(template_names, &options.resource_dir);
        all_errors.par_extend(errors);

        // 2/4: Priority joins between patches may allow templates to be processed in a parallel loop.
        let patches = { merge_patches(template_patch_map, &ids)? };

        // 3/4: Apply patches & Replace variables to indexes
        options.report_status(Status::ApplyingPatches);
        if let Err(errors) = apply_patches(&templates, patches) {
            all_errors.par_extend(errors);
        };
        let apply_errors_len = all_errors.len();

        // 4/4: Generate hkx files.
        options.report_status(Status::GenerateHkxFiles);
        let hkx_errors_len =
            if let Err(hkx_errors) = generate_hkx_files(options.output_dir, templates, ptr_map) {
                let errors_len = hkx_errors.len();
                all_errors.par_extend(hkx_errors);
                errors_len
            } else {
                0
            };

        if !all_errors.is_empty() {
            write_errors(&error_output, &all_errors).await?;
            return Err(Error::FailedToGenerateBehaviors {
                hkx_errors_len,
                patch_errors_len,
                apply_errors_len,
            });
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    #[cfg_attr(
        feature = "tracing",
        quick_tracing::init(file = "../../dummy/merge_test.log", stdio = false)
    )]
    async fn merge_test() {
        #[allow(clippy::iter_on_single_items)]
        let ids = [
            "../../dummy/Data/Nemesis_Engine/mod/aaaaa",
            "../../dummy/Data/Nemesis_Engine/mod/bcbi",
            "../../dummy/Data/Nemesis_Engine/mod/cbbi",
            "../../dummy/Data/Nemesis_Engine/mod/gender",
            "../../dummy/Data/Nemesis_Engine/mod/hmce",
            "../../dummy/Data/Nemesis_Engine/mod/momo",
            "../../dummy/Data/Nemesis_Engine/mod/na1w",
            "../../dummy/Data/Nemesis_Engine/mod/nemesis",
            "../../dummy/Data/Nemesis_Engine/mod/pscd",
            "../../dummy/Data/Nemesis_Engine/mod/rthf",
            "../../dummy/Data/Nemesis_Engine/mod/skice",
            "../../dummy/Data/Nemesis_Engine/mod/sscb",
            "../../dummy/Data/Nemesis_Engine/mod/tkuc",
            "../../dummy/Data/Nemesis_Engine/mod/tudm",
            "../../dummy/Data/Nemesis_Engine/mod/turn",
            "../../dummy/Data/Nemesis_Engine/mod/zcbe",
        ]
        .into_par_iter()
        .map(|s| s.into())
        .collect();

        behavior_gen(
            ids,
            Config {
                resource_dir: "../../resource/assets/templates".into(),
                output_dir: "../../dummy/behavior_gen/output".into(),
                status_report: None,
            },
        )
        .await
        .unwrap();
    }
}
