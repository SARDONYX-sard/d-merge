//! `cargo x gen behavior-table`: FNIS behavior table + templates -> `dummy/behaviors_table.json`.
//!
//! Reads `dummy/debug/FNIS_output/fnis_table.json` (dumped from FNIS) and looks up, in each
//! template xml, the indexes `generated_behaviors.rs` needs (root state, string data, ...).
//! The output is the input of `cargo x gen behaviors` (after adding `anim_data_key` by hand).

use std::path::Path;

pub(crate) fn run(ws: &Path) -> crate::Result {
    use std::fs;

    use havok_classes::Classes;
    use rayon::prelude::*;

    // -------------------------------
    // Input structures (from json)
    // -------------------------------
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    pub(crate) struct Root {
        pub version: String,
        pub columns: Vec<Entry>,
        pub creatures: Vec<Entry>,
        pub skeletons: Vec<Entry>,
        pub auxbones: Vec<Entry>,
        #[serde(rename = "plants/activators")]
        pub plants_activators: Vec<Entry>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    pub(crate) struct Entry {
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
    pub(crate) struct NewRoot {
        pub creatures: Vec<NewEntry>,
        pub skeletons: Vec<NewEntry>,
        pub auxbones: Vec<NewEntry>,
        #[serde(rename = "plants/activators")]
        pub plants_activators: Vec<NewEntry>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    pub(crate) struct NewEntry {
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
        /// `hkbVariableValueSet` index
        pub master_value_set_index: String,
    }

    impl NewEntry {
        fn from_indexes(entry: &Entry, indexes: BehaviorIndexes) -> Self {
            Self {
                behavior_object: entry.behavior_object.clone(),
                base_folder: entry.base_folder.replace("\\", "/"),
                default_behavior: entry.default_behavior.replace("\\", "/"),
                default_behavior_index: indexes.default_root_index,
                master_behavior: entry.master_behavior.replace("\\", "/"),
                master_behavior_index: indexes.master_root_index,
                master_string_data_index: indexes.master_string_data_index,
                master_behavior_graph_index: indexes.master_behavior_graph_index,
                master_value_set_index: indexes.master_value_set_index,
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

    struct BehaviorIndexes {
        default_root_index: String,
        master_root_index: String,
        master_string_data_index: String,
        master_value_set_index: String,
        master_behavior_graph_index: String,
    }

    // -------------------------------
    // Behavior analyzers
    // -------------------------------
    fn get_behavior(root_dir: &Path, entry: &Entry) -> Result<BehaviorIndexes, String> {
        let mut master = root_dir.join(&entry.base_folder).join(&entry.master_behavior);
        master.set_extension("xml");
        let mut default = root_dir.join(&entry.base_folder).join(&entry.default_behavior);
        default.set_extension("xml");

        let default_root_index = get_default_root_state(&default)?;
        let master_root_index = get_master_root_behavior(&master)
            .ok_or_else(|| format!("master not found: {}", master.display()))?;
        let (master_behavior_graph_index, master_string_data_index, master_value_set_index) =
            get_master_behavior_graph_data(&master)
                .ok_or_else(|| format!("master_info not found: {}", master.display()))?;

        Ok(BehaviorIndexes {
            default_root_index,
            master_root_index,
            master_string_data_index,
            master_value_set_index,
            master_behavior_graph_index,
        })
    }

    fn get_default_root_state(default: &Path) -> Result<String, String> {
        let string = std::fs::read_to_string(default)
            .map_err(|e| format!("failed to read default file {}: {}", default.display(), e))?;

        let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string)
            .map_err(|e| format!("failed to parse default file {}: {}", default.display(), e))?;

        let class: Vec<_> = class_map
            .par_iter()
            .filter_map(|(_, class)| match class {
                Classes::hkbCharacterStringData(class) => Some(class),
                _ => None,
            })
            .collect();

        if class.is_empty() || class.len() > 2 {
            return Err(format!(
                "unexpected number of hkbCharacterStringData classes: {}",
                class.len()
            ));
        }

        class[0]
            .__ptr
            .as_ref()
            .map(|ptr| ptr.to_string())
            .ok_or_else(|| "default_root_index pointer is None".to_string())
    }

    fn get_master_root_behavior(master: &Path) -> Option<String> {
        let string = std::fs::read_to_string(master).ok()?;
        let class_map: serde_hkx_features::ClassMap = serde_hkx::from_str(&string).ok()?;

        let (_, root) = class_map
            .par_iter()
            .find_first(|(_, class)| matches!(class, Classes::hkRootLevelContainer(_)))?;

        if let Classes::hkRootLevelContainer(container) = root
            && let Some(variant) = container.m_namedVariants.first()
        {
            let ptr = &variant.m_variant;
            if let Some(Classes::hkbBehaviorGraph(graph)) = class_map.get(ptr.get()) {
                return Some(graph.m_rootGenerator.to_string());
            }
        }

        None
    }

    /// Return (behavior index, hkbBehaviorGraphStringData index, hkbVariableValueSet index)
    fn get_master_behavior_graph_data(master: &Path) -> Option<(String, String, String)> {
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

        Some((
            class[0].__ptr.as_ref()?.to_string(),
            class[0].m_stringData.get().to_string(),
            class[0].m_variableInitialValues.get().to_string(),
        ))
    }

    let root_dir = &ws.join("resource/xml/templates/meshes");
    let mut errors: Vec<String> = Vec::new();

    // -------------------------------
    // Load source json
    // -------------------------------
    let parsed: Root = {
        let data = std::fs::read_to_string(ws.join("dummy/debug/FNIS_output/fnis_table.json"))?;
        serde_json::from_str(&data)?
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
    let json = serde_json::to_string_pretty(&new_root)?;
    fs::write(ws.join("dummy/behaviors_table.json"), json)?;

    if !errors.is_empty() {
        let joined = errors.join("\n");
        fs::write(ws.join("dummy/behaviors_table_errors.log"), joined)?;
    }
    Ok(())
}
