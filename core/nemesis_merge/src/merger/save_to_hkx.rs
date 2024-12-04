//! Processes a list of Nemesis XML paths and generates JSON output in the specified directory.
#![allow(clippy::mem_forget)]
use super::TemplateMap;
use crate::error::{Error, FailedIoSnafu, Result};
use rayon::prelude::*;
use serde_hkx::bytes::serde::hkx_header::HkxHeader;
use serde_hkx_features::ClassMap;
use simd_json::serde::from_borrowed_value;
use snafu::ResultExt;

pub(crate) fn save_templates_to_hkx(templates: TemplateMap<'_>) -> Result<(), Vec<Error>> {
    use std::fs;

    let results: Vec<Result<(), Error>> = templates
        .par_iter()
        .map(|key_value_ref_mut| {
            let (output_path, template_json) = key_value_ref_mut.value();

            let output_path_clone = output_path.clone();
            let template_json_clone = template_json.clone();

            if let Some(output_dir_all) = output_path_clone.parent() {
                fs::create_dir_all(output_dir_all).context(FailedIoSnafu {
                    path: output_dir_all,
                })?;
            }

            match from_borrowed_value::<ClassMap>(template_json_clone) {
                Ok(ast) => match serde_hkx::to_bytes(&ast, &HkxHeader::new_skyrim_se()) {
                    Ok(hkx_bytes) => {
                        fs::write(&output_path_clone, hkx_bytes).context(FailedIoSnafu {
                            path: output_path_clone,
                        })?;
                    }
                    Err(err) => return Err(err.into()),
                },
                Err(err) => {
                    return Err(Error::JsonError {
                        source: err,
                        path: output_path_clone,
                    })
                }
            }

            Ok(())
        })
        .collect();

    let errors: Vec<Error> = results.into_par_iter().filter_map(Result::err).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
