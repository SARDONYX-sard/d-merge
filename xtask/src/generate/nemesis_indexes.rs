//! `cargo x gen nemesis-indexes`: indexes of `hkbBehaviorGraphStringData` / `hkbBehaviorGraphData`
//! in each 3rd person template -> `dummy/extracted_indexes.json` (for checking the template tables).

use std::path::Path;

pub(crate) fn run(ws: &Path) -> crate::Result {
    use std::{collections::BTreeMap, fs};

    use behavior_merge_types::template_key::NEMESIS_3RD_PERSON_MAP;
    use havok_classes::Classes;
    use rayon::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Extracted {
        string_data_index: Option<String>,
        variable_data_index: Option<String>,
    }

    let root_dir = &ws.join("resource/xml/templates");
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
            let file_stem =
                xml_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

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

    let json = serde_json::to_string_pretty(&results)?;
    fs::write(ws.join("dummy/extracted_indexes.json"), json)?;

    if !errors.is_empty() {
        fs::write(ws.join("dummy/extracted_errors.log"), errors.join("\n"))?;
    }
    Ok(())
}
