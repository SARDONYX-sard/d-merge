//! The Nemesis path consists of the following
//!
//! - Format: `Nemesis_Engine/mod/<id>/[1st_person/]<template name>/<patch index>.txt`
//! - e.g.: `/some/path/to/Nemesis_Engine/mod/flinch/1st_person/0_master/#0106.txt`.
//!
//! From here, we need to determine and load the xml to which the patch will be applied, so we use a table to load this.
//! Note that the dir paths above `mesh` can be optionally specified and concatenated as resource paths later.
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Return HashMap<template key, `meshes` inner path>
pub fn collect_owned_templates<P>(path: P) -> dashmap::DashMap<String, PathBuf>
where
    P: AsRef<Path>,
{
    jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|path| template_name_and_inner_path(&path.ok()?.path()))
        .collect()
}

/// Return (hashmap key, meshes inner path)
fn template_name_and_inner_path(path: &Path) -> Option<(String, PathBuf)> {
    let is_xml = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));

    // Currently only supports XML in the `behaviors` dir.
    // This is the standard supported template for Nemesis.
    // TODO: Other than this, specifications need to be determined. `Defaultmale.xml` and others have duplicate filenames.
    let is_behaviors = path.components().any(|c| {
        let c = c.as_os_str();
        c.eq_ignore_ascii_case("behaviors")
            || c.eq_ignore_ascii_case("behaviors wolf")
            // || c.eq_ignore_ascii_case("character assets female")
            // || c.eq_ignore_ascii_case("character assets")
            // || c.eq_ignore_ascii_case("characterassets")
            || c.eq_ignore_ascii_case("characters female")
            || c.eq_ignore_ascii_case("characters")
    });
    if !is_behaviors || !is_xml {
        return None;
    }

    let key = {
        let file_stem = path.file_stem()?.to_string_lossy();

        let mut components = path.components();
        let is_1stperson = components.any(|c| c.as_os_str().eq_ignore_ascii_case("_1stPerson"));
        if is_1stperson {
            if path.to_str().is_some_and(|s| s.contains("\\")) {
                format!("_1stperson\\{file_stem}")
            } else {
                format!("_1stperson/{file_stem}")
            }
        } else {
            file_stem.to_string()
        }
    };

    let inner_path = {
        let mut value = None;
        let mut components = path.components();

        while let Some(comp) = components.next() {
            if comp.as_os_str().eq_ignore_ascii_case("mesh")
                || comp.as_os_str().eq_ignore_ascii_case("meshes")
            {
                let mut rel_path = PathBuf::from(comp.as_os_str()); // Include `mesh` too.
                rel_path.extend(components);
                value = Some(rel_path);
                break;
            }
        }
        value?
    };

    Some((key, inner_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1st_person() {
        let path =
            Path::new("resource/assets/templates/meshes/actors/character/behaviors/weapequip.xml");
        match template_name_and_inner_path(path) {
            Some((key, value)) => {
                assert_eq!(key, "weapequip");
                assert_eq!(
                    value,
                    Path::new("meshes/actors/character/behaviors/weapequip.xml")
                );
            }
            None => panic!("Invalid path"),
        }
    }

    #[test]
    fn test_1st_person_character() {
        let path = Path::new(
            "resource/assets/templates/meshes/actors/character/_1stperson/characters/firstperson.xml",
        );
        match template_name_and_inner_path(path) {
            Some((key, value)) => {
                assert_eq!(key, "_1stperson/firstperson");
                assert_eq!(
                    value,
                    Path::new("meshes/actors/character/_1stperson/characters/firstperson.xml")
                );
            }
            None => panic!("Invalid path"),
        }
    }

    #[ignore = "local only checker"]
    #[test]
    fn test_overwrite_path() {
        let path = "../../resource/assets";

        let mut map = indexmap::IndexMap::new();
        let mut overwrite = vec![];
        for result in jwalk::WalkDir::new(path) {
            let path = result.unwrap().path();

            if !path.is_file() {
                continue;
            }

            let Some((template_name, inner_path)) = template_name_and_inner_path(&path) else {
                continue;
            };

            if let Some(prev) = map.insert(template_name, inner_path.clone()) {
                overwrite.push((inner_path, prev));
            };
        }

        dbg!(&overwrite);
        dbg!(overwrite.len());
    }

    /// Return HashMap<template key, `meshes` inner path>
    pub fn collect_owned_templates<P>(path: P) -> Vec<PathBuf>
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
            // 残りの部分を使って新しいファイル名を作成
            let rest = &stem[prefix_len..];
            let new_file_name = format!("{}.{}", rest, ext);
            let mut new_path = path.to_path_buf();
            new_path.set_file_name(new_file_name);
            Some(new_path)
        } else {
            None
        }
    }
    /// `"meshes"` というディレクトリ名以降のパスを取得する（case-insensitive, ASCII）
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
                result.push("meshes"); // "meshes" 自体も含める
            }
        }

        if found {
            Some(result)
        } else {
            None
        }
    }

    #[ignore = "Local only"]
    #[test]
    fn local_create_bin_templates() {
        use crate::templates::collect::template_xml_to_value_inner;

        let output_dir = Path::new("./");
        let nemesis_map = {
            let nemesis_path = r"";
            let creature_path = r"";

            let mut nemesis_map = collect_owned_templates(nemesis_path);
            let creature_map = collect_owned_templates(creature_path);
            nemesis_map.par_extend(creature_map);
            nemesis_map
        };
        nemesis_map.into_par_iter().for_each(|path| {
            let Some(inner_path) = get_meshes_relative_path(&path) else {
                return;
            };
            let Some(inner_path) = remove_nemesis_prefix(&inner_path) else {
                return;
            };
            let Ok(value) = template_xml_to_value_inner(&path) else {
                return;
            };
            let Ok(bin) = rmp_serde::to_vec(&value) else {
                return;
            };

            let mut output_path = output_dir.join(&inner_path);
            output_path.set_extension("bin");

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&output_path, bin).unwrap();
        });
    }
}
