use std::borrow::Cow;

use rayon::prelude::*;

use crate::asdsf::{
    alt::{alt_key::to_normal_txt_project_name, AltAsdsf},
    normal::ser::write_anim_set,
};
use crate::diff_line::DiffLines;

const NEW_LINE: &str = "\r\n";

/// Patch for the txt array inside Root txt (e.g., `DefaultmaleData~Defaultmale`).
/// Resolves conflicts during key enumeration immediately before serialization.
///
/// e.g., - key: `DefaultMaleData~DefaultMale`, - value: diff lines
pub type SubHeaderDiffMap<'a> = std::collections::HashMap<&'a str, DiffLines<'a>>;

/// Converts an `AltAsdsf` struct back into the original `animationsetdatasinglefile.txt` text format with `\r\n` line endings.
///
/// # Errors
/// - Failed to apply patches.
pub fn serialize_alt_asdsf(
    alt_asdsf: AltAsdsf<'_>,
    patches: DiffLines,
    mut sub_txt_header_patch_map: SubHeaderDiffMap<'_>,
) -> Result<String, SerializeError> {
    let mut out = String::new();

    let mut txt_projects: Vec<_> = alt_asdsf
        .txt_projects
        .0
        .par_iter()
        .map(|(k, _)| {
            let mut out = String::new();
            if to_normal_txt_project_name(k, &mut out).is_none() {
                // This should not occur as long as we are using vanilla's asdsf.
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to convert path: {k}");
            }
            std::borrow::Cow::Owned(out)
        })
        .collect();

    patches.into_apply(&mut txt_projects)?;

    out.push_str(&txt_projects.len().to_string());
    out.push_str(NEW_LINE);
    for project_name in &txt_projects {
        out.push_str(project_name);
        out.push_str(NEW_LINE);
    }

    for vanilla_name in txt_projects {
        let mut name = String::new();
        if super::alt_key::to_alt_txt_project_name(vanilla_name.as_ref(), &mut name).is_none() {
            return Err(SerializeError::InvalidVanillaTxtProjectPath {
                name: vanilla_name.to_string(),
            });
        };
        let Some(anim_set_list) = alt_asdsf.txt_projects.0.get(name.as_str()) else {
            return Err(SerializeError::MissingTxtProjectHeader { name });
        };

        // sub header seq patch
        let mut sub_txt_headers: Vec<_> =
            anim_set_list.0.par_iter().map(|(k, _)| k.clone()).collect();
        if let Some(sub_header_diff) = sub_txt_header_patch_map.remove(name.as_str()) {
            sub_header_diff.into_apply(&mut sub_txt_headers)?;
        }
        write_file_names(&mut out, &sub_txt_headers);

        for sub_txt_header in sub_txt_headers {
            let Some(anim_set) = anim_set_list.0.get(sub_txt_header.as_ref()) else {
                return Err(SerializeError::MissingSubTxtHeader {
                    name,
                    sub_txt_name: sub_txt_header.to_string(),
                });
            };
            write_anim_set(&mut out, anim_set);
        }
    }

    Ok(out)
}

fn write_file_names(out: &mut String, sub_txt_headers: &[Cow<'_, str>]) {
    let file_names_len = sub_txt_headers.len();
    if file_names_len == 0 {
        return;
    }

    out.push_str(&file_names_len.to_string());
    out.push_str(NEW_LINE);

    for name in sub_txt_headers {
        out.push_str(name);
        out.push_str(NEW_LINE);
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum SerializeError {
    #[snafu(transparent)]
    DiffLine {
        source: crate::diff_line::error::Error,
    },

    /// Expected: `{name}.txt`. but  got none.
    MissingTxtProjectHeader { name: String },

    /// Expected: `{name}/{sub_txt_name}.txt`. but  got none.
    MissingSubTxtHeader { name: String, sub_txt_name: String },

    /// Failed to convert a vanilla-style txt project path into an alternative
    /// `~`-separated identifier.
    ///
    /// Expected format:
    /// `<folder>\<file>.txt`
    ///
    /// Examples:
    /// - `DefaultMaleData\DefaultMale.txt`
    ///
    /// Actual path:
    /// `{path}`
    #[snafu(display(
        r#"Failed to convert a vanilla-style txt project path into an alternative `~`-separated identifier.

Expected format:
`<folder>\<file>.txt`

Examples:
- `DefaultMaleData\DefaultMale.txt`

Actual path: `{name}`"#
    ))]
    InvalidVanillaTxtProjectPath { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asdsf::normal::de::parse_asdsf;

    fn normalize_to_crlf(input: &str) -> std::borrow::Cow<'_, str> {
        if input.contains("\r\n") {
            input.into()
        } else {
            input.replace("\r", "").replace("\n", "\r\n").into()
        }
    }

    #[cfg(feature = "alt_map")]
    #[test]
    fn test_serialize_asdsf() {
        let expected = normalize_to_crlf(include_str!(
            "../../../../../resource/xml/templates/meshes/animationsetdatasinglefile.txt"
        ));
        let asdsf = parse_asdsf(&expected).unwrap_or_else(|e| panic!("{e}"));
        let alt_asdsf = asdsf.try_into().unwrap_or_else(|e| panic!("{e}"));

        // std::fs::write("../../dummy/debug/adsf_debug.txt", format!("{:#?}", adsf)).unwrap();
        let actual =
            serialize_alt_asdsf(alt_asdsf, DiffLines::DEFAULT, SubHeaderDiffMap::new()).unwrap();

        // std::fs::create_dir_all("../../dummy").unwrap();
        // std::fs::write("../../dummy/adsf.txt", &actual).unwrap();

        let res = dbg!(actual == expected);
        if !res {
            let diff = ::diff::diff(&actual, &expected);
            std::fs::write("../../dummy/diff.txt", diff).unwrap();
            panic!("actual != expected");
        }
        assert!(res);
    }

    #[cfg(feature = "alt_map")]
    #[test]
    fn should_write_alt_asdsf_json() {
        let input = include_str!(
            "../../../../../resource/xml/templates/meshes/animationsetdatasinglefile.txt"
        );
        let asdsf = crate::asdsf::normal::de::parse_asdsf(input).unwrap_or_else(|err| {
            panic!("Failed to parse asdsf:\n{err}");
        });
        let alt_asdsf: AltAsdsf = asdsf.try_into().unwrap_or_else(|e| panic!("{e}"));

        std::fs::create_dir_all("../../dummy/debug/").unwrap();
        let json = serde_json::to_string_pretty(&alt_asdsf).unwrap_or_else(|err| {
            panic!("Failed to serialize adsf to JSON:\n{err}");
        });
        std::fs::write("../../dummy/debug/animationsetdatasinglefile.json", json).unwrap();

        let bin = rmp_serde::to_vec(&alt_asdsf).unwrap();
        std::fs::write("../../dummy/debug/animationsetdatasinglefile.bin", bin).unwrap();
    }
}
