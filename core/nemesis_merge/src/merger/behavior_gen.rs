//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use super::{
    apply_patches::apply_patches_to_templates,
    collect_templates_and_patches::collect_templates_and_patches,
    save_to_hkx::save_templates_to_hkx, Options,
};
use crate::error::{Error, FailedIoSnafu, Result};
use rayon::prelude::*;
use snafu::ResultExt;
use std::path::{Path, PathBuf};

/// - nemesis_paths: `e.g. vec!["../../dummy/Data/Nemesis_Engine/mod/aaaaa"]`
/// - `resource_dir`: Path of the template from which the patch was applied.(e.g. `../templates/` => `../templates/meshes`)
///
/// # Errors
/// Returns an error if file parsing, I/O operations, or JSON serialization fails.
pub async fn behavior_gen(nemesis_paths: Vec<PathBuf>, options: Options) -> Result<()> {
    let mut errors: Vec<Error> = vec![];
    let error_output = options.output_dir.join("errors.txt");

    let (templates, patch_mod_map) = match collect_templates_and_patches(nemesis_paths, options) {
        Ok((new_templates, new_patch_mod_map)) => (new_templates, new_patch_mod_map),
        Err(errors) => {
            let errors_len = errors.len();
            write_errors(&error_output, errors).await?;
            return Err(Error::FailedToReadTemplateAndPatches { errors_len });
        }
    };
    {
        // Lifetime inversion hack: `templates` require `patch_mod_map` to live longer than `templates`, but `templates` actually live longer than `templates`.
        // Therefore, reassign the local variable in the block to shorten the lifetime
        let templates = templates;

        // TODO: Priority joins between patches may allow templates to be processed in a parallel loop.
        // 2. apply patches
        let results = apply_patches_to_templates(&templates, &patch_mod_map);
        errors.extend(
            results
                .into_par_iter()
                .filter_map(Result::err)
                .map(|e| e)
                .collect::<Vec<Error>>(),
        );

        if let Err(errs) = save_templates_to_hkx(templates) {
            errors.extend(errs);
        };
    };
    write_errors(&error_output, errors).await?;

    Ok(())
}

async fn write_errors(path: impl AsRef<Path>, errors: Vec<Error>) -> Result<()> {
    let path = path.as_ref();

    let errors: Vec<String> = errors.into_par_iter().map(|e| e.to_string()).collect();
    tokio::fs::write(&path, errors.join("\n"))
        .await
        .context(FailedIoSnafu {
            path: path.to_path_buf(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "unimplemented yet"]
    #[quick_tracing::init]
    async fn merge_test() {
        #[allow(clippy::iter_on_single_items)]
        let ids = [
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\aaaaa",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\bcbi",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\cbbi",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\gender",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\hmce",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\momo",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\na1w",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\nemesis",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\pscd",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\rthf",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\skice",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\sscb",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\tkuc",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\tudm",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\turn",
            // "../../dummy\\Data\\Nemesis_Engine\\mod\\zcbe",
        ]
        .into_iter()
        .map(|s| s.into())
        .collect();

        behavior_gen(
            ids,
            Options {
                resource_dir: "../../assets/templates".into(),
                output_dir: "../../dummy/patches_applied/output".into(),
            },
        )
        .await
        .unwrap();
    }
}
