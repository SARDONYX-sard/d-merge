use rayon::prelude::*;

use crate::{
    asdsf::{
        alt::{alt_key::to_normal_txt_project_name, AltAsdsf, AltTxtProjects},
        normal::ser::{write_anim_set, write_file_names},
    },
    diff_line::DiffLines,
};

const NEW_LINE: &str = "\r\n";

/// Converts an `AltAsdsf` struct back into the original `animationsetdatasinglefile.txt` text format with `\r\n` line endings.
///
/// # Errors
/// Failed to apply patches.
pub fn serialize_alt_asdsf_with_patches(
    alt_asdsf: AltAsdsf<'_>,
    patches: DiffLines,
) -> Result<String, crate::diff_line::error::Error> {
    let mut out = String::new();

    let (mut txt_projects, anim_set_lists): (Vec<_>, Vec<_>) = alt_asdsf
        .txt_projects
        .0
        .into_par_iter()
        .map(|(k, v)| {
            let mut out = String::new();
            if to_normal_txt_project_name(&k, &mut out).is_none() {
                // This should not occur as long as we are using vanilla's asdsf.
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to convert path: {k}");
            }
            (k, v)
        })
        .unzip();
    patches.into_apply(&mut txt_projects)?;

    out.push_str(&txt_projects.len().to_string());
    out.push_str(NEW_LINE);
    for project_name in &txt_projects {
        out.push_str(project_name);
        out.push_str(NEW_LINE);
    }

    for anim_set_list in &anim_set_lists {
        write_file_names(&mut out, anim_set_list);
        for (_, anim_set) in &anim_set_list.0 {
            write_anim_set(&mut out, anim_set);
        }
    }

    Ok(out)
}

/// Converts an `AltAsdsf` struct back into the original `animationsetdatasinglefile.txt` text format with `\r\n` line endings.
pub fn serialize_alt_asdsf(data: &AltAsdsf<'_>) -> String {
    let mut out = String::new();

    write_projects(&mut out, &data.txt_projects);

    for (_, anim_set_list) in &data.txt_projects.0 {
        write_file_names(&mut out, anim_set_list);
        for (_, anim_set) in &anim_set_list.0 {
            write_anim_set(&mut out, anim_set);
        }
    }

    out
}

fn write_projects(out: &mut String, projects: &AltTxtProjects) {
    out.push_str(&projects.0.len().to_string());
    out.push_str(NEW_LINE);
    for (project_name, _) in &projects.0 {
        if to_normal_txt_project_name(project_name, out).is_none() {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to convert path: {project_name}");
        }
        out.push_str(NEW_LINE);
    }
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
        let actual = serialize_alt_asdsf(&alt_asdsf);

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
