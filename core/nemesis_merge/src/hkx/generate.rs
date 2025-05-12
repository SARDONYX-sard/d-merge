//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use crate::{
    aliases::BorrowedTemplateMap,
    errors::{Error, FailedIoSnafu, JsonSnafu, Result},
    results::filter_results,
};
use rayon::prelude::*;
use serde_hkx::bytes::serde::hkx_header::HkxHeader;
use serde_hkx_features::ClassMap;
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;
use std::{fs, path::Path};

pub(crate) fn generate_hkx_files(
    output_dir: impl AsRef<Path>,
    templates: BorrowedTemplateMap<'_>,
) -> Result<(), Vec<Error>> {
    let output_dir = output_dir.as_ref();

    let results = templates
        .into_par_iter()
        .map(|(_, (inner_path, template_json))| {
            let mut output_path = output_dir.join(inner_path);

            if let Some(output_dir_all) = output_path.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            #[cfg(feature = "debug")]
            write_json_patch(&output_path, &template_json)?;

            let hkx_bytes = {
                let class_map =
                    from_borrowed_value::<ClassMap>(template_json).with_context(|_| JsonSnafu {
                        path: output_path.clone(),
                    })?;

                let header = HkxHeader::new_skyrim_se();

                // create id_maps
                // hkx ids
                //
                // 0_master.hkx: $eventNames[speed]$
                // $name$.hkx:   $eventNames[speed]$

                // let ser = serde_hkx::bytes::ser::ByteSerializer::from_maps(
                //     // event_id_map,
                //     // variable_id_map,
                //     &header,
                // );
                serde_hkx::bytes::ser::to_bytes(&class_map, &header)?
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
        simd_json::to_string_pretty(&template_json).context(crate::error::JsonSnafu {
            path: json_path.clone(),
        })?,
    )
    .context(FailedIoSnafu {
        path: json_path.clone(),
    })?;

    Ok(())
}
