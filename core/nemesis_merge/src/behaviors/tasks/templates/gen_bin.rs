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

    #[test]
    fn test_gen_behaviors() {
        let paths = crate::behaviors::tasks::fnis::collect::collect_paths(
            "../../dummy/templates/bins/**/behaviors/*.bin",
        )
        .unwrap();
        std::fs::write("./behaviors.log", format!("{paths:#?}")).unwrap();
    }

    #[test]
    fn test_gen_behaviors_table() {
        use havok_classes::Classes;

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

        let parsed: Root = {
            let mut data =
                std::fs::read_to_string("../../dummy/debug/FNIS_output/fnis_table.json").unwrap();
            simd_json::from_slice(unsafe { data.as_bytes_mut() }).unwrap()
        };

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
        }

        impl NewEntry {
            fn from_indexes(entry: &Entry, indexes: (String, String)) -> Self {
                Self {
                    behavior_object: entry.behavior_object.clone(),
                    base_folder: entry.base_folder.clone(),
                    default_behavior: entry.default_behavior.clone(),
                    default_behavior_index: indexes.1,
                    master_behavior: entry.master_behavior.clone(),
                    master_behavior_index: indexes.0,
                }
            }
        }

        let root_dir = r"../dummy/hkxcmd_xml/meshes";

        let new_root = NewRoot {
            creatures: parsed
                .creatures
                .par_iter()
                .map(|entry| NewEntry::from_indexes(entry, get_behavior(root_dir, entry)))
                .collect(),
            skeletons: parsed
                .skeletons
                .par_iter()
                .map(|entry| NewEntry::from_indexes(entry, get_behavior(root_dir, entry)))
                .collect(),
            auxbones: parsed
                .auxbones
                .par_iter()
                .map(|entry| NewEntry::from_indexes(entry, get_behavior(root_dir, entry)))
                .collect(),
            plants_activators: parsed
                .plants_activators
                .par_iter()
                .map(|entry| NewEntry::from_indexes(entry, get_behavior(root_dir, entry)))
                .collect(),
        };

        fn get_behavior(root_dir: impl AsRef<Path>, entry: &Entry) -> (String, String) {
            let root_dir = root_dir.as_ref();

            // hkbStateMachine
            let mut master = Path::new(root_dir)
                .join(&entry.base_folder)
                .join(&entry.master_behavior);
            master.set_extension("xml");

            // find hkbCharacterStringData
            let mut default = Path::new(root_dir)
                .join(&entry.base_folder)
                .join(&entry.default_behavior);
            default.set_extension("xml");

            if let (Some(master), Some(default)) = (
                get_master_root_behavior(&master),
                get_default_root_state(&default),
            ) {
                return (master, default);
            };

            panic!(
                "Not found: master={}, default={}",
                master.display(),
                default.display()
            );
        }

        fn get_default_root_state(default: &Path) -> Option<String> {
            let string = std::fs::read_to_string(default).unwrap();
            let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string)
                .unwrap_or_else(|e| {
                    panic!("serde_hkx de error:{}\n {e}", default.display());
                });

            let class: Vec<_> = class_map
                .par_iter()
                .filter_map(|(_, class)| match class {
                    Classes::hkbCharacterStringData(class) => Some(class),
                    _ => None,
                })
                .collect();
            if class.is_empty() || class.len() > 2 {
                panic!("hkbCharacterStringData len {}", class.len());
            };

            class[0].__ptr.as_ref().map(|ptr| ptr.to_string())
        }

        fn get_master_root_behavior(master: &Path) -> Option<String> {
            // - master finder
            //   hkRootLevelContainer
            //   -> namedVariants[0]: ptr
            //   -> variant: ptr -> (map[ptr])
            //   -> hkbBehaviorGraph.rootGenerator
            let string = std::fs::read_to_string(master).unwrap();
            let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string)
                .unwrap_or_else(|e| {
                    panic!("serde_hkx de error:{}\n {e}", master.display());
                });

            let (_, root) = class_map
                .par_iter()
                .find_first(|(_, class)| matches!(class, Classes::hkRootLevelContainer(_)))
                .unwrap_or_else(|| {
                    panic!("Not found hkRootLevelContainer from {}", master.display())
                });

            if let Classes::hkRootLevelContainer(container) = root {
                // namedVariants[0].variant が BehaviorGraph を指す
                if let Some(variant) = container.m_namedVariants.first() {
                    let ptr = &variant.m_variant;

                    let behavior_graph = class_map.get(ptr.get()).unwrap();
                    if let Classes::hkbBehaviorGraph(graph) = behavior_graph {
                        // rootGenerator を取る
                        return Some(graph.m_rootGenerator.to_string());
                    }
                }
            }

            None
        }

        let json = simd_json::to_string_pretty(&new_root)
            .unwrap_or_else(|_| panic!("simd_json ser error"));
        std::fs::write("./behaviors_table.json", json).unwrap();
    }
}
