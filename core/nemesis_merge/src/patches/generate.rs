use crate::errors::{
    Error, FailedIoSnafu, JsonSnafu, MissingParseNemesisPathSnafu, NemesisXmlErrSnafu, Result,
};
use crate::paths::collect::collect_nemesis_paths;
use crate::results::filter_results;
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::prelude::*;
use snafu::{OptionExt, ResultExt};
use std::path::{Path, PathBuf};

/// Generate nemesis patches to json patches.
/// # Errors
/// IF failed to parse nemesis file.
pub fn generate_patches<A, P>(patch_path: A, output: P) -> Result<(), Vec<Error>>
where
    A: AsRef<Path>,
    P: AsRef<Path>,
{
    let output = output.as_ref();

    let results: Vec<_> = collect_nemesis_paths(patch_path)
        .par_iter()
        .map(|txt_path| -> Result<()> {
            let json = {
                let nemesis_xml =
                    std::fs::read_to_string(txt_path).with_context(|_| FailedIoSnafu {
                        path: txt_path.clone(),
                    })?;
                let patch =
                    parse_nemesis_patch(&nemesis_xml).with_context(|_| NemesisXmlErrSnafu {
                        path: txt_path.clone(),
                    })?;
                simd_json::to_string_pretty(&patch).with_context(|_| JsonSnafu {
                    path: txt_path.clone(),
                })?
            };

            let mut output = {
                let start_index = txt_path
                    .iter()
                    .position(|component| component.eq_ignore_ascii_case("Nemesis_Engine"))
                    .with_context(|| MissingParseNemesisPathSnafu { path: txt_path })?;
                let relevant_path: PathBuf = txt_path.iter().skip(start_index + 2).collect();
                output.join(relevant_path)
            };
            output.set_extension("json");
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent).with_context(|_| FailedIoSnafu { path: parent })?;
            }
            std::fs::write(&output, json).with_context(|_| FailedIoSnafu { path: output })?;

            Ok(())
        })
        .collect();

    filter_results(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "need test data"]
    fn test_generate_patches() {
        let output = Path::new("../../dummy/behavior_gen/patches");
        if let Err(errors) = generate_patches("../../dummy/Data/", output) {
            let errors: Vec<String> = errors.into_par_iter().map(|err| err.to_string()).collect();
            std::fs::write(output.join("errors.txt"), errors.join("\n")).unwrap();
        }
    }
}
