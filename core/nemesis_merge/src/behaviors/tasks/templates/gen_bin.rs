use crate::behaviors::tasks::templates::collect::borrowed::template_xml_to_value;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Create `.bin` from `.xml` template.
/// - `paths`: `meshes` parent dir.
///
/// # Examples
/// ```no_run
/// let paths = "../resource/templates/default/
/// ../resource/templates/creatures/";
///
/// let output_dir = Path::new("../../dummy/templates/bins");
/// nemesis_merge::create_bin_templates(paths.split("\n"), output_dir);
/// ```
pub fn create_bin_templates<I, P>(paths: I, output_dir: &Path)
where
    I: Iterator<Item = P>,
    P: AsRef<Path>,
{
    let paths = paths.flat_map(collect_templates);
    paths.for_each(|path| {
        let f = || -> Option<()> {
            let inner_path = get_meshes_relative_path(&path)?;
            let inner_path = remove_nemesis_prefix(&inner_path)?;
            let bytes = std::fs::read(&path).ok()?;
            let value = template_xml_to_value(&bytes, &path).ok()?;
            let bin = rmp_serde::to_vec(&value).ok()?;

            let mut output_path = output_dir.join(&inner_path);
            output_path.set_extension("bin");
            if let Some(parent) = output_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&output_path, bin).ok()
        };

        if f().is_none() {
            println!("Failed path: {}", path.display());
        }
    });
}

/// Return HashMap<template key, `meshes` inner path>
fn collect_templates<P>(path: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|path| {
            let path = path.ok()?.path();
            if !path.is_file() {
                return None;
            }

            Some(path)
        })
        .collect()
}

fn remove_nemesis_prefix(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_string_lossy();
    let ext = path.extension()?.to_string_lossy();

    let prefix = "nemesis_";
    let prefix_len = prefix.len();

    if stem.len() >= prefix_len && stem[..prefix_len].eq_ignore_ascii_case(prefix) {
        let rest = &stem[prefix_len..];
        let mut new_path = path.to_path_buf();
        let new_file_name = format!("{rest}.{ext}");
        new_path.set_file_name(new_file_name);
        Some(new_path)
    } else {
        Some(path.to_path_buf())
    }
}

fn get_meshes_relative_path(path: &Path) -> Option<PathBuf> {
    let components = path.components();

    let mut found = false;
    let mut result = PathBuf::new();

    for component in components {
        let s = component.as_os_str().to_string_lossy();
        if found {
            result.push(component.as_os_str());
        } else if s.eq_ignore_ascii_case("meshes") {
            found = true;
            result.push("meshes");
        }
    }

    if found {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ignore = "local only"]
    #[test]
    fn test_create_bin_templates() {
        // let paths = std::fs::read_to_string("../../dummy/templates_paths.txt").unwrap();
        let paths = ["../../resource/xml"];
        let output_dir = Path::new("../../dummy/templates/bins");
        create_bin_templates(paths.iter(), output_dir);
    }
}
