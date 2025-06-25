//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
use crate::{
    behaviors::tasks::{
        patches::types::BehaviorStringDataMap, templates::types::BorrowedTemplateMap,
    },
    config::{ReportType, StatusReportCounter},
    errors::{Error, FailedIoSnafu, HkxSerSnafu, JsonSnafu, Result},
    results::filter_results,
    Config, OutPutTarget,
};
use rayon::prelude::*;
use serde_hkx::{bytes::serde::hkx_header::HkxHeader, EventIdMap, HavokSort as _, VariableIdMap};
use serde_hkx_features::{
    id_maker::crate_maps_from_id_class as create_maps_from_id_class, ClassMap,
};
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn generate_hkx_files<'a: 'b, 'b>(
    config: &Config,
    templates: BorrowedTemplateMap<'a>,
    variable_class_map: BehaviorStringDataMap<'b>,
) -> Result<(), Vec<Error>> {
    let reporter = StatusReportCounter::new(
        &config.status_report,
        ReportType::GeneratingHkxFiles,
        templates.len(),
    );

    let results = templates
        .into_par_iter()
        .map(|(key, (inner_path, template_json))| {
            reporter.increment();
            let mut output_path = config.output_dir.join(inner_path);

            if let Some(output_dir_all) = output_path.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            let hkx_bytes = {
                if config.debug.output_merged_json {
                    let debug_path = debug_file_path(&config.output_dir, inner_path);
                    write_patched_json(&debug_path, &template_json)?;
                }

                let mut class_map: ClassMap =
                    from_borrowed_value(template_json).with_context(|_| JsonSnafu {
                        path: output_path.clone(),
                    })?;

                if config.debug.output_merged_xml {
                    let debug_path = debug_file_path(&config.output_dir, inner_path);
                    write_patched_xml(&debug_path, &class_map)?;
                };

                let mut event_id_map = None;
                let mut variable_id_map = None;
                if let Some(pair) = variable_class_map.0.get(&key) {
                    let ptr = pair.value();

                    // Create eventID & variableId maps from hkbBehaviorGraphStringData class
                    if let Some((event_map, var_map)) = class_map
                        .get(*ptr)
                        .and_then(|class| create_maps_from_id_class(class))
                    {
                        event_id_map = Some(event_map);
                        variable_id_map = Some(var_map);
                    };
                }

                // Convert to hkx bytes & Replace nemesis id.
                let header = match config.output_target {
                    OutPutTarget::SkyrimLe => HkxHeader::new_skyrim_le(),
                    OutPutTarget::SkyrimSe => HkxHeader::new_skyrim_se(),
                };
                let event_id_map = event_id_map.unwrap_or_else(EventIdMap::new);
                let variable_id_map = variable_id_map.unwrap_or_else(VariableIdMap::new);

                // NOTE: T-pause if we don't sort before `to_bytes`.
                class_map.sort_for_bytes();

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

fn debug_file_path(output_dir: &Path, inner_path: &str) -> PathBuf {
    output_dir.join(".d_merge").join(".debug").join(inner_path)
}

/// Output template.json & template.json debug string
pub fn write_patched_json<S>(output_file: &Path, template_json: S) -> Result<()>
where
    S: serde::Serialize + core::fmt::Debug,
{
    if let Some(output_dir_all) = output_file.parent() {
        fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
            path: output_dir_all,
        })?;
    }
    if let Ok(pretty_json) = simd_json::to_string_pretty(&template_json) {
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
