pub mod tables;

use crate::error::{FailedIoSnafu, JsonSnafu, NemesisXmlErrSnafu, Result};
use nemesis_xml::patch::parse_nemesis_patch;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use winnow::Parser;

/// - `ids`: Nemesis xml paths.
///
/// # Errors
/// Failed to parse patch
#[allow(clippy::result_large_err)]
pub fn behavior_gen(output_dir: impl AsRef<Path>, ids: Vec<String>) -> Result<()> {
    let output_dir = output_dir.as_ref();

    let mut result = Vec::new();

    for id in ids {
        let paths: Vec<PathBuf> = jwalk::WalkDir::new(id)
            .into_iter()
            .filter_map(|res| {
                if let Ok(path) = res.map(|entry| entry.path()) {
                    let file_name = path.file_stem()?.to_str()?;
                    let is_nemesis_file = nemesis_xml::helpers::tag::index_name
                        .parse(file_name)
                        .is_ok();
                    if path.is_file() && is_nemesis_file {
                        return Some(path);
                    }
                }
                None
            })
            .collect();

        let res: Vec<_> = paths
            .par_iter()
            .map(|path| -> Result<()> {
                let input = std::fs::read_to_string(path).context(FailedIoSnafu { path })?; // TODO: use `read _any_string` of `serde_hkx_features`
                let json_patch = parse_nemesis_patch(&input).context(NemesisXmlErrSnafu {
                    xml: input.clone(),
                    path,
                })?;

                let output = {
                    let mut output = generate_output_path(path, output_dir)
                        .unwrap_or_else(|| output_dir.to_path_buf());
                    output.set_extension("json");

                    if let Some(output_dir) = output.parent() {
                        std::fs::create_dir_all(output_dir)
                            .context(FailedIoSnafu { path: output_dir })?;
                    }

                    output
                };

                let json_str = simd_json::to_string_pretty(&json_patch).context(JsonSnafu {
                    path: output.clone(),
                })?;
                std::fs::write(&output, json_str).context(FailedIoSnafu { path: output })?;

                Ok(())
            })
            .collect();
        result.extend(res);
    }

    let errors: Vec<String> = result
        .into_iter()
        .filter_map(Result::err)
        .map(|err| err.to_string())
        .collect();
    if !errors.is_empty() {
        std::fs::create_dir_all(output_dir).context(FailedIoSnafu { path: output_dir })?;
        let output = output_dir.join("errors.txt");
        std::fs::write(&output, errors.join("\n")).context(FailedIoSnafu { path: output })?;
    }

    Ok(())
}

fn generate_output_path(input_path: &Path, output_dir: &Path) -> Option<PathBuf> {
    let input_inner_dir = input_path
        .strip_prefix(input_path.ancestors().nth(3)?)
        .ok()?
        .components()
        .collect::<PathBuf>();

    let output = output_dir.join(input_inner_dir);
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "unimplemented yet"]
    #[test]
    fn merge_test() {
        // "Nemesis_Engine/mod/flinch/0_master/#0106.txt";
        // "Nemesis_Engine/mod/flinch/0_master/#flinch$0.txt";
        let ids = [
            "../../dummy\\Data\\Nemesis_Engine\\mod\\aaaaa",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\bcbi",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\cbbi",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\gender",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\hmce",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\momo",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\na1w",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\nemesis",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\pscd",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\rthf",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\skice",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\sscb",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\tkuc",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\tudm",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\turn",
            "../../dummy\\Data\\Nemesis_Engine\\mod\\zcbe",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
        let output_dir = "../../dummy/patches/output";
        std::fs::create_dir_all(output_dir).unwrap();

        behavior_gen(output_dir, ids).unwrap();
    }

    #[test]
    fn generate_output_path_test() {
        let input_path = Path::new("/some/path/to/file.ext");
        let output_dir = Path::new("/output");

        assert_eq!(
            generate_output_path(input_path, output_dir),
            Some(Path::new("/output/path/to/file.ext").to_path_buf())
        );
    }
}
