//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    config::{Config, Status},
    errors::{write_errors::write_errors, Error, Result},
    hkx::generate::generate_hkx_files,
    patches::{
        apply::apply_patches,
        collect::{collect_borrowed_patches, collect_owned_patches, BorrowedPatches},
    },
    paths::id::paths_to_priority_map,
    templates::collect::collect_templates,
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

    let mut all_errors = vec![];

    // 1/4: Collect all patches & templates xml
    config.on_report_status(Status::ReadingTemplatesAndPatches);
    let (owned_adsf_patches, owned_patches) =
        match collect_owned_patches(&nemesis_paths, &id_order).await {
            Ok(owned_patches) => owned_patches,
            Err(errors) => {
                let errors_len = errors.len();

                let err = Error::FailedToReadOwnedPatches { errors_len };
                config.on_report_status(Status::Error(err.to_string()));

                write_errors(&config, &errors).await?;
                return Err(err);
            }
        };

    let adsf_errors = crate::adsf::apply_adsf_patches(owned_adsf_patches, &id_order, &config);
    let adsf_errors_len = adsf_errors.len();
    all_errors.par_extend(adsf_errors);

    // 2/4: Priority joins between patches may allow templates to be processed in a parallel loop.
    let (
        BorrowedPatches {
            template_names,
            patch_map_foreach_template,
            variable_class_map,
        },
        patch_errors_len,
    ) = {
        let (patch_result, errors) = collect_borrowed_patches(&owned_patches, config.hack_options);
        let patch_errors_len = errors.len();
        all_errors.par_extend(errors);
        (patch_result, patch_errors_len)
    };

    // HACK: Lifetime inversion hack: `templates` require `patch_mod_map` to live longer than `templates`, but `templates` actually live longer than `templates`.
    // Therefore, reassign the local variable in the block to shorten the lifetime
    {
        let (templates, errors) = collect_templates(template_names, &config.resource_dir);
        all_errors.par_extend(errors);

        // 3/4: Apply patches & Replace variables to indexes
        config.on_report_status(Status::ApplyingPatches);
        let mut apply_errors_len = 0;
        if let Err(errors) =
            apply_patches(&templates, patch_map_foreach_template, &config.output_dir)
        {
            apply_errors_len = errors.len();
            all_errors.par_extend(errors);
        };

        // 4/4: Generate hkx files.
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

        if !all_errors.is_empty() {
            write_errors(&config, &all_errors).await?;

            let err = Error::FailedToGenerateBehaviors {
                adsf_errors_len,
                hkx_errors_len,
                patch_errors_len,
                apply_errors_len,
            };

            config.on_report_status(Status::Error(err.to_string()));
            return Err(err);
        };
    };

    config.on_report_status(Status::Done);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    #[cfg(feature = "tracing")]
    async fn merge_test() {
        use nemesis_xml::hack::HackOptions;

        let log_path = "../../dummy/merge_test.log";
        crate::global_logger::global_logger(log_path, tracing::Level::TRACE).unwrap();

        #[allow(clippy::iter_on_single_items)]
        let ids = [
            // "../../dummy/Data/Nemesis_Engine/mod/aaaaa",
            // "../../dummy/Data/Nemesis_Engine/mod/bcbi",
            // "../../dummy/Data/Nemesis_Engine/mod/cbbi",
            // "../../dummy/Data/Nemesis_Engine/mod/gender",
            // "../../dummy/Data/Nemesis_Engine/mod/hmce",
            // "../../dummy/Data/Nemesis_Engine/mod/momo",
            // "../../dummy/Data/Nemesis_Engine/mod/na1w",
            // "../../dummy/Data/Nemesis_Engine/mod/nemesis",
            // "../../dummy/Data/Nemesis_Engine/mod/pscd",
            // "../../dummy/Data/Nemesis_Engine/mod/rthf",
            // "../../dummy/Data/Nemesis_Engine/mod/skice",
            // "../../dummy/Data/Nemesis_Engine/mod/sscb",
            // "../../dummy/Data/Nemesis_Engine/mod/tkuc",
            // "../../dummy/Data/Nemesis_Engine/mod/tudm",
            // "../../dummy/Data/Nemesis_Engine/mod/turn",
            // "../../dummy/Data/Nemesis_Engine/mod/zcbe",
            "D:/GAME/ModOrganizer Skyrim SE/mods/FlinchingSSE やられモーションを追加(要OnHit/Nemesis_Engine/mod/flinch",
            // "D:/GAME/ModOrganizer Skyrim SE/mods/Crouch Sliding スプリント→しゃがみでスライディング/Nemesis_Engine/mod/slide",
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
                hack_options: Some(HackOptions::enable_all()),
            },
        )
        .await
        .unwrap();
    }
}
