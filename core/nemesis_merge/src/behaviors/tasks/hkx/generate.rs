//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use std::{
    fs,
    path::{Path, PathBuf},
};

use rayon::prelude::*;
use serde_hkx::{EventIdMap, HavokSort as _, VariableIdMap, bytes::serde::hkx_header::HkxHeader};
use serde_hkx_features::{ClassMap, id_maker::create_maps};
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;

use crate::{
    Config, OutPutTarget,
    behaviors::tasks::{
        patches::types::BehaviorGraphDataMap, templates::types::BorrowedTemplateMap,
    },
    config::{ReportType, StatusReportCounter},
    errors::{Error, FailedIoSnafu, HkxSerSnafu, JsonToClassMapSnafu, Result},
    results::filter_results,
};

pub(crate) fn generate_hkx_files(
    config: &Config,
    templates: BorrowedTemplateMap<'_>,
    variable_class_map: BehaviorGraphDataMap<'_>,
) -> Result<(), Vec<Error>> {
    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::GeneratingHkxFiles,
        templates.len(),
    );

    let results = templates
        .into_par_iter()
        .map(|(key, template_json)| {
            reporter.increment();
            let inner_path = key.as_meshes_inner_path();
            let mut output_path = config.output_dir.join(inner_path);

            if let Some(output_dir_all) = output_path.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            let hkx_bytes = {
                // The error occurring with the following `from_borrowed_value` indicates that the intended JSON
                // format has not been achieved, suggesting an issue lies with the tool itself.
                if config.debug.output_merged_json {
                    let debug_path = debug_file_path(&config.output_dir, inner_path);
                    write_patched_json(&debug_path, &template_json)?;
                }

                let mut class_map: ClassMap =
                    from_borrowed_value(template_json).with_context(|_| JsonToClassMapSnafu {
                        path: output_path.clone(),
                    })?;

                let mut event_id_map = None;
                let mut variable_id_map = None;
                if let Some(pair) = variable_class_map.0.get(&key) {
                    let master_behavior_graph_index = pair;

                    // Deduplication is prone to unexpected index misalignments, and there is no guarantee that this will not occur.
                    // Furthermore, since values with the same name are synchronized, there are no drawbacks other than wasted resources.
                    //
                    // It checks whether the number of values associated with an event matches the number of default values associated with a value name.
                    // This is done to prevent crashes before they occur.
                    use serde_hkx_features::id_maker::check_len_from_map;
                    check_len_from_map(&class_map, &master_behavior_graph_index).with_context(
                        |_| crate::errors::DedupEventVariableSnafu {
                            path: output_path.clone(),
                        },
                    )?;

                    // NOTE: Since we will no longer be able to use `&mut` on `class_map` after this point, we must call it here.
                    class_map.sort_for_bytes(); // NOTE: If we don't sort hkx by dependency order, a T/A pose will occur.

                    if let Some((event_map, var_map)) =
                        create_maps(&class_map, &master_behavior_graph_index)
                    {
                        event_id_map = Some(event_map);
                        variable_id_map = Some(var_map);
                    };
                } else {
                    class_map.sort_for_bytes(); // NOTE: To avoid T/A pose.
                }

                // NOTE: View the debug output after removing duplicates. Otherwise, duplicate eventNames will appear.
                if config.debug.output_merged_xml {
                    let debug_path = debug_file_path(&config.output_dir, inner_path);
                    write_patched_xml(&debug_path, &class_map)?;
                };

                // Convert to hkx bytes & Replace nemesis id.
                let header = match config.output_target {
                    OutPutTarget::SkyrimLe => HkxHeader::new_skyrim_le(),
                    OutPutTarget::SkyrimSe => HkxHeader::new_skyrim_se(),
                };
                let event_id_map = event_id_map.unwrap_or_else(EventIdMap::new);
                let variable_id_map = variable_id_map.unwrap_or_else(VariableIdMap::new);

                // Output error info
                // serialize target class, field ptr number.
                serde_hkx::to_bytes_with_maps(&class_map, &header, event_id_map, variable_id_map)
                    .with_context(|_| HkxSerSnafu {
                        path: output_path.clone(),
                    })?
            };

            output_path.set_extension("hkx");
            fs::write(&output_path, hkx_bytes).with_context(|_| FailedIoSnafu {
                path: output_path.clone(),
            })?;

            #[cfg(feature = "tracing")]
            tracing::info!("Generated: {}", output_path.display());
            Ok(())
        })
        .collect();

    filter_results(results)
}

fn debug_file_path(output_dir: &Path, inner_path: &Path) -> PathBuf {
    output_dir.join(".d_merge").join(".debug").join(inner_path)
}

/// Output template.json & template.json debug string
pub(crate) fn write_patched_json<S>(output_file: &Path, template_json: S) -> Result<()>
where
    S: serde::Serialize + core::fmt::Debug,
{
    if let Some(output_dir_all) = output_file.parent() {
        fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
            path: output_dir_all,
        })?;
    }
    if let Ok(pretty_json) = sonic_rs::to_string_pretty(&template_json) {
        let mut json_path = output_file.to_path_buf();
        json_path.set_extension("json");
        fs::write(&json_path, pretty_json).context(FailedIoSnafu { path: json_path })?;
    } else {
        // If pretty print fails, fall back to normal print
        let mut debug_path = output_file.to_path_buf();
        debug_path.set_extension("debug_json.log");
        fs::write(&debug_path, format!("{template_json:#?}"))
            .context(FailedIoSnafu { path: debug_path })?;
    }

    Ok(())
}

fn write_patched_xml(output_path: &Path, class_map: &ClassMap<'_>) -> Result<()> {
    use serde_hkx::HavokSort as _;

    let mut class_map = class_map.clone();
    let ptr = class_map
        .sort_for_xml()
        .with_context(|_| HkxSerSnafu { path: output_path })?;
    let xml = serde_hkx::to_string(&class_map, &ptr)
        .with_context(|_| HkxSerSnafu { path: output_path })?;

    let mut xml_path = output_path.to_path_buf();
    xml_path.set_extension("xml");
    if let Some(output_dir_all) = xml_path.parent() {
        fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
            path: output_dir_all,
        })?;
    }
    fs::write(&xml_path, &xml).context(FailedIoSnafu {
        path: xml_path.clone(),
    })?;

    Ok(())
}
