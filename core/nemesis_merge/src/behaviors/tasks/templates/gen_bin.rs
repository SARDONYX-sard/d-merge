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
        // let paths = ["../../dummy/overwrited_xml"];
        let output_dir = Path::new("../../dummy/templates/bins");
        create_bin_templates(paths.iter(), output_dir);
    }

    #[ignore = "local only"]
    #[test]
    fn test_gen_behaviors() {
        let paths = crate::behaviors::tasks::fnis::collect::collect_paths(
            "../../dummy/templates/bins/**/behaviors/*.bin",
        )
        .unwrap();
        std::fs::write("./behaviors.log", format!("{paths:#?}")).unwrap();
    }

    #[ignore = "local only"]
    #[test]
    fn test_gen_behaviors_table() {
        use havok_classes::Classes;
        use rayon::prelude::*;
        use std::{fs, path::Path};

        // -------------------------------
        // Input structures (from json)
        // -------------------------------
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub struct Root {
            pub version: String,
            pub columns: Vec<Entry>,
            pub creatures: Vec<Entry>,
            pub skeletons: Vec<Entry>,
            pub auxbones: Vec<Entry>,
            #[serde(rename = "plants/activators")]
            pub plants_activators: Vec<Entry>,
        }

        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub struct Entry {
            /// "behavior_object": "riekling",
            pub behavior_object: String,
            /// "base_folder": "actors\\dlc02\\riekling",
            pub base_folder: String,
            /// "default_behavior": "characters\\rieklingcharacter.hkx",
            pub default_behavior: String,
            /// "master_behavior": "behaviors\\rieklingbehavior.hkx",
            pub master_behavior: String,
            /// "back_to_default_event": "returnToDefault",
            pub back_to_default_event: String,
            /// "DLC_depending": "",
            #[serde(rename = "DLC_depending")]
            pub dlc_depending: String,
            /// "special_AOUnequip": "",
            #[serde(rename = "special_AOUnequip")]
            pub special_aounequip: String,
            /// "Skip_CC": "",
            #[serde(rename = "Skip_CC")]
            pub skip_cc: String,
            #[serde(rename = "BO_Id")]
            /// "BO_Id": "33",
            pub bo_id: String,
            /// "BO_Anims": "88"
            #[serde(rename = "BO_Anims")]
            pub bo_anims: String,
        }

        // -------------------------------
        // Output structures
        // -------------------------------
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub struct NewRoot {
            pub creatures: Vec<NewEntry>,
            pub skeletons: Vec<NewEntry>,
            pub auxbones: Vec<NewEntry>,
            #[serde(rename = "plants/activators")]
            pub plants_activators: Vec<NewEntry>,
        }

        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        pub struct NewEntry {
            /// "behavior_object": "riekling",
            pub behavior_object: String,
            /// "base_folder": "actors\\dlc02\\riekling",
            pub base_folder: String,
            /// "default_behavior": "characters\\rieklingcharacter.hkx",
            pub default_behavior: String,
            /// #0200
            pub default_behavior_index: String,
            /// "master_behavior": "behaviors\\rieklingbehavior.hkx",
            pub master_behavior: String,
            /// #0100
            pub master_behavior_index: String,
            /// `hkbBehaviorGraphStringData` index. e.g. `#0106`, _1stperson `#0095`
            pub master_string_data_index: String,
            /// `hkbBehaviorGraphData` index. e.g. `#0108`
            pub master_behavior_graph_index: String,
        }

        impl NewEntry {
            fn from_indexes(entry: &Entry, indexes: (String, String, String, String)) -> Self {
                Self {
                    behavior_object: entry.behavior_object.clone(),
                    base_folder: entry.base_folder.replace("\\", "/"),
                    default_behavior: entry.default_behavior.replace("\\", "/"),
                    default_behavior_index: indexes.0,
                    master_behavior: entry.master_behavior.replace("\\", "/"),
                    master_behavior_index: indexes.1,
                    master_string_data_index: indexes.2,
                    master_behavior_graph_index: indexes.3,
                }
            }
        }

        // -------------------------------
        // Common processor for all groups
        // -------------------------------
        fn process_entries(
            entries: &[Entry],
            root_dir: &Path,
            errors: &mut Vec<String>,
        ) -> Vec<NewEntry> {
            entries
                .par_iter()
                .map(|entry| match get_behavior(root_dir, entry) {
                    Ok(indexes) => Ok(NewEntry::from_indexes(entry, indexes)),
                    Err(e) => Err(format!("{}: {}", entry.behavior_object, e)),
                })
                .collect::<Vec<_>>()
                .into_iter()
                .filter_map(|res| match res {
                    Ok(v) => Some(v),
                    Err(e) => {
                        errors.push(e);
                        None
                    }
                })
                .collect()
        }

        // -------------------------------
        // Behavior analyzers
        // -------------------------------
        fn get_behavior(
            root_dir: &Path,
            entry: &Entry,
        ) -> Result<(String, String, String, String), String> {
            let mut master = root_dir
                .join(&entry.base_folder)
                .join(&entry.master_behavior);
            master.set_extension("xml");
            let mut default = root_dir
                .join(&entry.base_folder)
                .join(&entry.default_behavior);
            default.set_extension("xml");

            let default_res = get_default_root_state(&default)
                .ok_or_else(|| format!("default not found: {}", default.display()))?;
            let master_res = get_master_root_behavior(&master)
                .ok_or_else(|| format!("master not found: {}", master.display()))?;
            let master_string_data_res = get_master_string_data(&master)
                .ok_or_else(|| format!("master_var not found: {}", master.display()))?;
            let master_behavior_graph_res = get_master_behavior_graph_data(&master)
                .ok_or_else(|| format!("master_info not found: {}", master.display()))?;

            Ok((
                default_res,
                master_res,
                master_string_data_res,
                master_behavior_graph_res,
            ))
        }

        fn get_default_root_state(default: &Path) -> Option<String> {
            let string = std::fs::read_to_string(default).ok()?;
            let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string).ok()?;

            let class: Vec<_> = class_map
                .par_iter()
                .filter_map(|(_, class)| match class {
                    Classes::hkbCharacterStringData(class) => Some(class),
                    _ => None,
                })
                .collect();

            if class.is_empty() || class.len() > 2 {
                return None;
            }

            class[0].__ptr.as_ref().map(|ptr| ptr.to_string())
        }

        fn get_master_root_behavior(master: &Path) -> Option<String> {
            let string = std::fs::read_to_string(master).ok()?;
            let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string).ok()?;

            let (_, root) = class_map
                .par_iter()
                .find_first(|(_, class)| matches!(class, Classes::hkRootLevelContainer(_)))?;

            if let Classes::hkRootLevelContainer(container) = root {
                if let Some(variant) = container.m_namedVariants.first() {
                    let ptr = &variant.m_variant;
                    if let Some(Classes::hkbBehaviorGraph(graph)) = class_map.get(ptr.get()) {
                        return Some(graph.m_rootGenerator.to_string());
                    }
                }
            }

            let class: Vec<_> = class_map
                .par_iter()
                .filter_map(|(_, class)| match class {
                    Classes::hkbBehaviorGraphStringData(class) => Some(class),
                    _ => None,
                })
                .collect();

            if class.is_empty() || class.len() > 2 {
                return None;
            }
            class[0].__ptr.as_ref().map(|ptr| ptr.to_string());

            None
        }

        fn get_master_string_data(master: &Path) -> Option<String> {
            let string = std::fs::read_to_string(master).ok()?;
            let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string).ok()?;

            let class: Vec<_> = class_map
                .par_iter()
                .filter_map(|(_, class)| match class {
                    Classes::hkbBehaviorGraphStringData(class) => Some(class),
                    _ => None,
                })
                .collect();

            if class.is_empty() || class.len() > 2 {
                return None;
            }
            class[0].__ptr.as_ref().map(|ptr| ptr.to_string())
        }

        fn get_master_behavior_graph_data(master: &Path) -> Option<String> {
            let string = std::fs::read_to_string(master).ok()?;
            let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string).ok()?;

            let class: Vec<_> = class_map
                .par_iter()
                .filter_map(|(_, class)| match class {
                    Classes::hkbBehaviorGraphData(class) => Some(class),
                    _ => None,
                })
                .collect();

            if class.is_empty() || class.len() > 2 {
                return None;
            }
            Some(class[0].__ptr.as_ref()?.to_string())
        }

        let root_dir = Path::new("../../resource/xml/templates/meshes");
        let mut errors: Vec<String> = Vec::new();

        // -------------------------------
        // Load source json
        // -------------------------------
        let parsed: Root = {
            let mut data =
                std::fs::read_to_string("../../dummy/debug/FNIS_output/fnis_table.json").unwrap();
            simd_json::from_slice(unsafe { data.as_bytes_mut() }).unwrap()
        };

        // -------------------------------
        // Build output
        // -------------------------------
        let new_root = NewRoot {
            creatures: process_entries(&parsed.creatures, root_dir, &mut errors),
            skeletons: process_entries(&parsed.skeletons, root_dir, &mut errors),
            auxbones: process_entries(&parsed.auxbones, root_dir, &mut errors),
            plants_activators: process_entries(&parsed.plants_activators, root_dir, &mut errors),
        };

        // -------------------------------
        // Write outputs
        // -------------------------------
        let json = simd_json::to_string_pretty(&new_root).unwrap();
        fs::write("../../dummy/behaviors_table.json", json).unwrap();

        if !errors.is_empty() {
            let joined = errors.join("\n");
            fs::write("../../dummy/fnis/table/errors_list.log", joined).unwrap();
        }
    }

    #[ignore = "local only"]
    #[test]
    fn test_extract_indexes() {
        use crate::behaviors::tasks::templates::key::NEMESIS_3RD_PERSON_MAP;
        use havok_classes::Classes;
        use rayon::prelude::*;
        use serde::{Deserialize, Serialize};
        use std::{collections::BTreeMap, fs, path::Path};

        #[derive(Debug, Serialize, Deserialize)]
        struct Extracted {
            string_data_index: Option<String>,
            variable_data_index: Option<String>,
        }

        let root_dir = Path::new("../../resource/xml/templates");
        let mut errors: Vec<String> = Vec::new();

        // output: file_stem, indexes
        let mut results: BTreeMap<String, Extracted> = BTreeMap::new();

        NEMESIS_3RD_PERSON_MAP
            .values()
            // NEMESIS_1ST_PERSON_MAP
            //     .values()
            .copied()
            .for_each(|rel_path| {
                // bin → xml
                let mut xml_path = root_dir.join(rel_path);
                xml_path.set_extension("xml");
                let file_stem = xml_path.file_stem().unwrap().to_string_lossy().to_string();

                match std::fs::read_to_string(&xml_path) {
                    Ok(content) => {
                        match serde_hkx::from_str::<serde_hkx_features::ClassMap>(&content) {
                            Ok(class_map) => {
                                let string_classes: Vec<_> = class_map
                                    .par_iter()
                                    .filter_map(|(_, class)| match class {
                                        Classes::hkbBehaviorGraphStringData(c) => c.__ptr.clone(),
                                        _ => None,
                                    })
                                    .map(|ptr| ptr.to_string())
                                    .collect();

                                let variable_classes: Vec<_> = class_map
                                    .par_iter()
                                    .filter_map(|(_, class)| match class {
                                        Classes::hkbBehaviorGraphData(c) => c.__ptr.clone(),
                                        _ => None,
                                    })
                                    .map(|ptr| ptr.to_string())
                                    .collect();

                                if string_classes.len() > 1 {
                                    errors.push(format!(
                                        "{}: multiple hkbBehaviorGraphStringData found",
                                        xml_path.display()
                                    ));
                                    return;
                                }
                                if variable_classes.len() > 1 {
                                    errors.push(format!(
                                        "{}: multiple hkbBehaviorGraphData found",
                                        xml_path.display()
                                    ));
                                    return;
                                }

                                results.insert(
                                    file_stem,
                                    Extracted {
                                        string_data_index: string_classes.first().cloned(),
                                        variable_data_index: variable_classes.first().cloned(),
                                    },
                                );
                            }
                            Err(e) => {
                                errors.push(format!(
                                    "serde_hkx parse error in {}: {}",
                                    xml_path.display(),
                                    e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("read error in {}: {}", xml_path.display(), e));
                    }
                }
            });

        let json = simd_json::to_string_pretty(&results).unwrap();
        fs::write("../../dummy/extracted_indexes.json", json).unwrap();

        if !errors.is_empty() {
            fs::write("../../dummy/extracted_errors.log", errors.join("\n")).unwrap();
        }
    }
}
