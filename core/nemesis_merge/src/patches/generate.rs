use crate::aliases::SortedPatchMap;
use crate::errors::{
    Error, FailedIoSnafu, JsonSnafu, MissingParseNemesisPathSnafu, NemesisXmlErrSnafu, Result,
};
use crate::paths::collect::collect_nemesis_paths;
use crate::results::filter_results;
use json_patch::{JsonPatch, JsonPath};
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
    let _ = std::fs::create_dir_all(output);

    let results: Vec<_> = collect_nemesis_paths(patch_path)
        .par_iter()
        .map(|txt_path| txt_to_json(txt_path, output))
        .collect();

    filter_results(results)
}

/// # Why need this?
/// The specification was changed to change key to path array in HashMap, so it is no longer possible to convert to json.
/// Therefore, this structure is used to make it an output-ready format.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
struct GenerableJsonPatch<'a> {
    //  HashMap<Vec<Cow<'_, str>>, JsonPatch<'_>>
    path: JsonPath<'a>,
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "JsonPatch<'a>: serde::Deserialize<'de>"))
    )]
    value: JsonPatch<'a>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
struct GenerableJsonPatches<'a>(
    #[cfg_attr(
        feature = "serde",
        serde(bound(deserialize = "Vec<GenerableJsonPatch<'a>>: serde::Deserialize<'de>"))
    )]
    Vec<GenerableJsonPatch<'a>>,
);

impl<'a> GenerableJsonPatches<'a> {
    fn from(value: SortedPatchMap<'a>) -> Self {
        let mut ret = vec![];
        for (key, value) in value {
            ret.push(GenerableJsonPatch { path: key, value });
        }
        Self(ret)
    }
}

fn txt_to_json(txt_path: &PathBuf, output: &Path) -> Result<()> {
    let json = {
        let nemesis_xml = std::fs::read_to_string(txt_path).with_context(|_| FailedIoSnafu {
            path: txt_path.clone(),
        })?;
        let patch = parse_nemesis_patch(&nemesis_xml).with_context(|_| NemesisXmlErrSnafu {
            path: txt_path.clone(),
        })?;

        let patch = GenerableJsonPatches::from(patch);
        simd_json::to_string_pretty(&patch).with_context(|_| JsonSnafu {
            path: txt_path.clone(),
        })?
    };

    let output = {
        let start_index = txt_path
            .iter()
            .position(|component| component.eq_ignore_ascii_case("Nemesis_Engine"))
            .with_context(|| MissingParseNemesisPathSnafu { path: txt_path })?;
        let relevant_path: PathBuf = txt_path.iter().skip(start_index + 2).collect();
        output.join(relevant_path)
    };

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).with_context(|_| FailedIoSnafu { path: parent })?;
    }
    std::fs::write(&output, json).with_context(|_| FailedIoSnafu { path: output })?;

    Ok(())
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
