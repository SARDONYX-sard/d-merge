//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use super::{aliases::BorrowedTemplateMap, results::filter_results};
use crate::error::{Error, FailedIoSnafu, Result};
use rayon::prelude::*;
use serde_hkx::bytes::serde::hkx_header::HkxHeader;
use serde_hkx_features::ClassMap;
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;
use std::fs;

pub(crate) fn generate_hkx_files(templates: BorrowedTemplateMap<'_>) -> Result<(), Vec<Error>> {
    let results = templates
        .into_par_iter()
        .map(|(_, (output_path, template_json))| {
            if let Some(output_dir_all) = output_path.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            #[cfg(feature = "debug")] // output template.json & template.json debug string
            {
                let mut json_path = output_path.clone();
                json_path.set_extension("json.log");
                fs::write(&json_path, format!("{template_json:#?}")).context(FailedIoSnafu {
                    path: json_path.clone(),
                })?;

                let mut json_path = output_path.clone();
                json_path.set_extension("json");
                fs::write(
                    &json_path,
                    simd_json::to_string_pretty(&template_json).context(
                        crate::error::JsonSnafu {
                            path: json_path.clone(),
                        },
                    )?,
                )
                .context(FailedIoSnafu {
                    path: json_path.clone(),
                })?;
            }

            type ClassMapAlt<'a> =
                indexmap::IndexMap<std::borrow::Cow<'a, str>, havok_classes::Classes<'a>>;
            match from_borrowed_value::<ClassMapAlt>(template_json) {
                Ok(ast) => {
                    let result: Result<ClassMap, _> = ast
                        .into_par_iter()
                        .map(|(key, value)| {
                            key.parse::<usize>().map(|parsed_key| (parsed_key, value))
                        })
                        .collect();
                    match result {
                        Ok(class_map) => {
                            match serde_hkx::to_bytes(&class_map, &HkxHeader::new_skyrim_se()) {
                                Ok(hkx_bytes) => {
                                    fs::write(&output_path, hkx_bytes)
                                        .context(FailedIoSnafu { path: output_path })?;
                                    #[cfg(feature = "tracing")]
                                    tracing::info!(
                                        "Generation complete: {}",
                                        output_path.display()
                                    );
                                }
                                Err(err) => return Err(err.into()),
                            }
                        }
                        Err(err) => {
                            return Err(Error::Custom {
                                msg: err.to_string(),
                            })
                        }
                    }
                }
                Err(err) => {
                    return Err(Error::JsonError {
                        source: err,
                        path: output_path,
                    })
                }
            }

            Ok(())
        })
        .collect();

    filter_results(results)
}
