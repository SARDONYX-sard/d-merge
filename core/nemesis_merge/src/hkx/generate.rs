//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use crate::{
    aliases::{BorrowedTemplateMap, PtrMap},
    errors::{Error, FailedIoSnafu, HkxSerSnafu, JsonSnafu, Result},
    results::filter_results,
};
use rayon::prelude::*;
use serde_hkx::{bytes::serde::hkx_header::HkxHeader, EventIdMap, VariableIdMap};
use serde_hkx_features::{id_maker::crate_maps_from_id_class, ClassMap};
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;
use std::{fs, path::Path};

pub(crate) fn generate_hkx_files(
    output_dir: impl AsRef<Path>,
    templates: BorrowedTemplateMap<'_>,
    ptr_map: PtrMap<'_>,
) -> Result<(), Vec<Error>> {
    let output_dir = output_dir.as_ref();

    let results = templates
        .into_par_iter()
        .map(|(file_stem, (inner_path, template_json))| {
            let mut output_path = output_dir.join(inner_path);

            if let Some(output_dir_all) = output_path.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            #[cfg(feature = "debug")]
            write_json_patch(&output_path, &template_json)?;

            let hkx_bytes = {
                let class_map: ClassMap =
                    from_borrowed_value(template_json).with_context(|_| JsonSnafu {
                        path: output_path.clone(),
                    })?;
                // #[cfg(feature = "debug")]
                // write_xml(&output_path, &class_map)?;

                let mut event_id_map = None;
                let mut variable_id_map = None;
                if let Some(pair) = ptr_map.0.get(&file_stem) {
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

                // #[cfg(feature = "tracing")]
                // {
                //     tracing::trace!("event_id_map = {:#?}", event_id_map);
                //     tracing::trace!("variable_id_map = {:#?}", variable_id_map);
                // }

                // Convert to hkx bytes & Replace nemesis id.
                let header = HkxHeader::new_skyrim_se();
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
            tracing::info!("Generation complete: {}", output_path.display());
            Ok(())
        })
        .collect();

    filter_results(results)
}

#[cfg(feature = "debug")] // output template.json & template.json debug string
/// Output template.json & template.json debug string
fn write_json_patch(
    output_path: &Path,
    template_json: &simd_json::BorrowedValue<'_>,
) -> Result<()> {
    let mut json_path = output_path.to_path_buf();
    json_path.set_extension("json.log");
    fs::write(&json_path, format!("{template_json:#?}")).context(FailedIoSnafu {
        path: json_path.clone(),
    })?;

    let mut json_path = output_path.to_path_buf();
    json_path.set_extension("json");
    fs::write(
        &json_path,
        simd_json::to_string_pretty(&template_json).context(crate::errors::JsonSnafu {
            path: json_path.clone(),
        })?,
    )
    .context(FailedIoSnafu {
        path: json_path.clone(),
    })?;

    Ok(())
}

#[cfg(feature = "debug")]
#[allow(unused)]
fn write_xml(output_path: &Path, class_map: &ClassMap<'_>) -> Result<()> {
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
