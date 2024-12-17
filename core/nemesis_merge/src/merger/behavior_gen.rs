//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use super::{
    apply_patches::apply_patches,
    collect_templates_and_patches::collect_templates_and_patches,
    config::{Config, Status},
    hkx_files_gen::generate_hkx_files,
    merge_patches::{merge_mod_patches, paths_to_ids},
    write_errors::write_errors,
};
use crate::error::{Error, Result};
use rayon::prelude::*;
use std::path::PathBuf;

/// - nemesis_paths: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(nemesis_paths: Vec<PathBuf>, options: Config) -> Result<()> {
    let error_output = options.output_dir.join("d_merge_errors.log");

    // 1/4:
    options.report_status(Status::ReadingTemplatesAndPatches);
    let (templates, mod_patch_map) = match collect_templates_and_patches(&nemesis_paths, &options) {
        Ok((templates, patch_mod_map)) => (templates, patch_mod_map),
        Err(errors) => {
            let errors_len = errors.len();
            write_errors(&error_output, &errors).await?;
            return Err(Error::FailedToReadTemplateAndPatches { errors_len });
        }
    };
    #[cfg(feature = "tracing")]
    tracing::debug!("mod_patch_map = {mod_patch_map:#?}");

    let ids = paths_to_ids(&nemesis_paths);
    {
        // HACK: Lifetime inversion hack: `templates` require `patch_mod_map` to live longer than `templates`, but `templates` actually live longer than `templates`.
        // Therefore, reassign the local variable in the block to shorten the lifetime
        let templates = templates;

        // 2/4: Priority joins between patches may allow templates to be processed in a parallel loop.
        let patches = { merge_mod_patches(&mod_patch_map, &ids)? };

        // 3/4: Apply patches
        options.report_status(Status::ApplyingPatches);
        let mut errors = match apply_patches(&templates, patches) {
            Ok(()) => vec![],
            Err(errors) => errors,
        };
        let apply_errors_len = errors.len();

        // 4/4: Generate hkx files.
        options.report_status(Status::GenerateHkxFiles);
        let errors_len = if let Err(hkx_errors) = generate_hkx_files(templates) {
            let errors_len = hkx_errors.len();
            errors.par_extend(hkx_errors);
            errors_len
        } else {
            0
        };

        if !errors.is_empty() {
            write_errors(&error_output, &errors).await?;
            return Err(Error::FailedToGenerateBehaviors {
                hkx_errors_len: errors_len,
                patch_errors_len: apply_errors_len,
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
