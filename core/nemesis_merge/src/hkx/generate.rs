//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use crate::{
    errors::{Error, FailedIoSnafu, HkxSerSnafu, JsonSnafu, Result},
    results::filter_results,
    types::{BorrowedTemplateMap, VariableClassMap},
};
use rayon::prelude::*;
use serde_hkx::{bytes::serde::hkx_header::HkxHeader, EventIdMap, HavokSort as _, VariableIdMap};
use serde_hkx_features::{id_maker::crate_maps_from_id_class, ClassMap};
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;
use std::{fs, path::Path};

pub(crate) fn generate_hkx_files<'a>(
    output_dir: &Path,
    templates: BorrowedTemplateMap<'a>,
    variable_class_map: VariableClassMap<'a>,
) -> Result<(), Vec<Error>> {
    let results = templates
        .into_par_iter()
        .map(|(file_stem, (inner_path, template_json))| {
            let mut output_path = output_dir.join(&inner_path);

            if let Some(output_dir_all) = output_path.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            #[cfg(feature = "debug")]
            let debug_path = {
                let debug_path = output_dir.join(".debug").join(inner_path);

                if let Some(output_dir_all) = debug_path.parent() {
                    fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                        path: output_dir_all,
                    })?;
                }

                debug_path
            };

            #[cfg(feature = "debug")]
            write_patched_json(&debug_path, &template_json)?;

            let hkx_bytes = {
                let mut class_map: ClassMap =
                    from_borrowed_value(template_json).with_context(|_| JsonSnafu {
                        path: output_path.clone(),
                    })?;

                // #[cfg(feature = "debug")]
                // write_patched_xml(&debug_path, &class_map)?;

                let mut event_id_map = None;
                let mut variable_id_map = None;
                if let Some(pair) = variable_class_map.0.get(&file_stem) {
                    let ptr = pair.value();

                    // Create eventID & variableId maps from hkbBehaviorGraphStringData class
                    if let Some((event_map, var_map)) = class_map
                        .get(*ptr)
                        .and_then(|class| crate_maps_from_id_class(class))
                    {
                        event_id_map = Some(event_map);
                        variable_id_map = Some(var_map);
                    };
                }

                // Convert to hkx bytes & Replace nemesis id.
                let header = HkxHeader::new_skyrim_se();
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

#[cfg(feature = "debug")]
/// Output template.json & template.json debug string
fn write_patched_json(
    output_base: &Path,
    template_json: &simd_json::BorrowedValue<'_>,
) -> Result<()> {
    if let Ok(pretty_json) = simd_json::to_string_pretty(&template_json) {
        let mut json_path = output_base.to_path_buf();
        json_path.set_extension("json");
        fs::write(&json_path, pretty_json).context(FailedIoSnafu { path: json_path })?;
    } else {
        // If pretty print fails, fall back to normal print
        let mut debug_path = output_base.to_path_buf();
        debug_path.set_extension("debug_json.log");
        fs::write(&debug_path, format!("{template_json:#?}"))
            .context(FailedIoSnafu { path: debug_path })?;
    }

    Ok(())
}

#[cfg(feature = "debug")]
#[allow(unused)]
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
    fs::write(&xml_path, &xml).context(FailedIoSnafu {
        path: xml_path.clone(),
    })?;

    Ok(())
}
